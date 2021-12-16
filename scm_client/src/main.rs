use clap::{App, Arg, ArgMatches, SubCommand};

use ethers::abi::{parse_abi, Abi};
use ethers::contract::Contract;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::core::types::{Address, U256};
use ethers::middleware::SignerMiddleware;
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::{LocalWallet, Signer};

use std::sync::Arc;

// TODO use lazy_static to init WETH/SCM/Provider?
// TODO in order to sign transactions as someone else
// read private key from environent or CLI

// TODO set gas properties for transactions, otherwise
// they will probably not get picked up

fn provider() -> Provider<Http> {
    Provider::<Http>::try_from("https://rinkeby.infura.io/v3/2dbea4f559d440e8bc4301a42368d298")
        .expect("infura url is malformed.")
    // Provider::<Http>::try_from("http://localhost:8545").expect("infura url is malformed.")
}

const WETH: &str = "0xc778417E063141139Fce010982780140Aa0cD5Ab";
const SCM: &str = "0xd19C61495084c4ebCe9e3Fa4140b3830957ef852";
const ICO: &str = "0xdFb81Af3E53B1b95814c518bc7d503Abc756Fe08";
//4 for Rinkeby
const CHAIN_ID: u64 = 4;

// NOTE: Getting the ABI from a .sol file with Solc::compile_source()
// requires a STABLE version of the solidity compiler accessable as
// solc. pacman -S solidity will only install a nigthly version and
// installing with npm will install solcjs NOT solc.
// Let's not deal with that by typing out the interface for now.
fn erc20_abi() -> Abi {
    parse_abi(&[
        "function balanceOf(address) view returns (uint256)",
        "function approve(address, uint256)",
    ])
    .expect("malformed ERC20 interface")
}

fn ico_abi() -> Abi {
    parse_abi(&[
        "function claim()",
        "function claimableScm(address) view returns (uint256)",
        "function isCompleted() view returns (bool)",
        "function invest(uint256)",
    ])
    .expect("malformed ICO interface")
}

fn weth_contract<M: Middleware>(provider: impl Into<Arc<M>>) -> anyhow::Result<Contract<M>> {
    let weth_address = WETH.parse::<Address>()?;
    Ok(Contract::new(weth_address, erc20_abi(), provider))
}

fn scm_contract<M: Middleware>(provider: impl Into<Arc<M>>) -> anyhow::Result<Contract<M>> {
    let scm_address = SCM.parse::<Address>()?;
    Ok(Contract::new(scm_address, erc20_abi(), provider))
}

fn ico_contract<M: Middleware>(provider: impl Into<Arc<M>>) -> anyhow::Result<Contract<M>> {
    let ico_address = ICO.parse::<Address>()?;
    Ok(Contract::new(ico_address, ico_abi(), provider))
}

async fn get_weth_balance(address: Address) -> anyhow::Result<()> {
    let weth = weth_contract(provider())?;
    let balance: U256 = weth.method("balanceOf", address)?.call().await?;
    println!("WETH balance: {} wei", balance);
    Ok(())
}

async fn get_scm_balance(address: Address) -> anyhow::Result<()> {
    let provider = provider();
    let scm = scm_contract(provider)?;
    let balance: U256 = scm.method("balanceOf", address)?.call().await?;
    println!("SCM balance: {} wei", balance);
    Ok(())
}

async fn get_claimable_scm(address: Address) -> anyhow::Result<()> {
    let provider = provider();
    let ico = ico_contract(provider.clone())?;
    let claimable_scm = ico
        .method::<_, U256>("claimableScm", address)?
        .call()
        .await?;
    println!("claimable SCM: {} wei", claimable_scm);
    Ok(())
}

async fn claim_scm(wallet: impl Signer) -> anyhow::Result<()> {
    let provider = provider();
    let ico = ico_contract(provider.clone())?;
    let client = SignerMiddleware::new(provider, wallet);

    let tx = ico.method::<_, ()>("claim", ())?.tx;
    client
        .send_transaction(tx, None)
        .await
        .map_err(|e| anyhow::anyhow!("can't send transaction: {}", e))?;
    Ok(())
}

async fn get_ico_status() -> anyhow::Result<()> {
    let provider = provider();
    let ico = ico_contract(provider)?;
    let is_completed: bool = ico.method("isCompleted", ())?.call().await?;
    println!("ICO completed: {}", is_completed);
    Ok(())
}

// as required this sends 2 transactions after each other without waiting
// for the first one to be included in the chain
async fn invest(wallet: impl Signer, amount: &str, unit: &str) -> anyhow::Result<()> {
    let wei = ethers::utils::parse_units(amount, unit)?;
    let provider = provider();
    let client = SignerMiddleware::new(provider.clone(), wallet);

    let ico = ico_contract(provider.clone())?;
    let weth = weth_contract(provider)?;

    let mut approval_tx = weth.method::<_, ()>("approve", (ico.address(), wei))?.tx;
    client
        .fill_transaction(&mut approval_tx, None)
        .await
        .map_err(|e| anyhow::anyhow!("can't fill fields of transaction: {}", e))?;

    let current_nonce = approval_tx
        .nonce()
        .expect("nonce has not been fillled")
        .clone();

    client
        .send_transaction(approval_tx, None)
        .await
        .map_err(|e| anyhow::anyhow!("can't send transaction: {}", e))?;

    let mut invest_tx = ico.method::<_, ()>("invest", wei)?.tx;
    // Set nonce manually to avoid reusing the nonce of the previous transaction.
    invest_tx.set_nonce(current_nonce + 1);
    // Set gas limit manually to avoid gas estimation with eth_call which would always
    // fail because the allowance has not yet been raised by the previous transaction
    invest_tx.set_gas(90_000u64);

    let pending_tx = client
        .send_transaction(invest_tx, None)
        .await
        .map_err(|e| anyhow::anyhow!("can't send transaction: {}", e))?;

    println!("waiting for inclusion in the chain");
    pending_tx.await?;

    println!("invested amount: {} {}", amount, unit);
    Ok(())
}

fn get_address_from_args(arguments: &ArgMatches) -> anyhow::Result<Address> {
    Ok(arguments
        .value_of("wallet")
        .expect("required by clap")
        .parse::<Address>()?)
}

fn get_signer_from_args(arguments: &ArgMatches) -> anyhow::Result<LocalWallet> {
    let key = arguments.value_of("key").expect("required by clap");
    // allow passing "0xab12.." as well as "ab12.." as the private key
    let stripped_key = if key.starts_with("0x") {
        &key[2..]
    } else {
        &key[..]
    };
    let signing_key = SigningKey::from_bytes(hex::decode(stripped_key)?.as_slice())?;
    // mainnet is the default chain so we have to override it
    Ok(LocalWallet::from(signing_key).with_chain_id(CHAIN_ID))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wallet_argument = Arg::with_name("wallet")
        .short("w")
        .takes_value(true)
        .value_name("address")
        .required(true);

    let key_argument = Arg::with_name("key")
        .short("k")
        .takes_value(true)
        .value_name("private key")
        .required(true);

    let app = App::new("Scm Client")
        .version("1.0")
        .about("Let's you interact with the SCM ICO and token")
        .subcommand(
            SubCommand::with_name("weth-balance")
                .about("Tells you how much WETH you own")
                .arg(wallet_argument.clone()),
        )
        .subcommand(
            SubCommand::with_name("scm-balance")
                .about("Tells you how much SCM you own")
                .arg(wallet_argument.clone()),
        )
        .subcommand(
            SubCommand::with_name("claimable-scm")
                .about("Tells you how much SCM you can claim")
                .arg(wallet_argument.clone()),
        )
        .subcommand(
            SubCommand::with_name("claim-scm")
                .about("Claim your hard earned SCM")
                .arg(key_argument.clone()),
        )
        .subcommand(SubCommand::with_name("ico-status").about("Queries status of ICO."))
        .subcommand(
            SubCommand::with_name("invest")
                .about("Invest in the ICO")
                .arg(key_argument.clone())
                .arg(
                    Arg::with_name("amount")
                        .short("a")
                        .takes_value(true)
                        .required(true)
                        .value_name("wei")
                        .help("How much to invest into SCM"),
                )
                .arg(
                    Arg::with_name("unit")
                        .short("u")
                        .takes_value(true)
                        .required(true)
                        .value_name("unit")
                        .help("Supply unit like wei, gwei, ether"),
                ),
        )
        .get_matches();

    match app.subcommand() {
        ("weth-balance", Some(arguments)) => {
            let address = get_address_from_args(arguments)?;
            get_weth_balance(address).await?;
        }
        ("scm-balance", Some(arguments)) => {
            let address = get_address_from_args(arguments)?;
            get_scm_balance(address).await?;
        }
        ("claimable-scm", Some(arguments)) => {
            let wallet = get_address_from_args(arguments)?;
            get_claimable_scm(wallet).await?;
        }
        ("ico-status", _) => {
            get_ico_status().await?;
        }
        ("invest", Some(arguments)) => {
            let amount = arguments.value_of("amount").expect("required by clap");
            let unit = arguments.value_of("unit").expect("required by clap");
            let wallet = get_signer_from_args(arguments)?;
            invest(wallet, amount, unit).await?;
        }
        ("claim-scm", Some(arguments)) => {
            let wallet = get_signer_from_args(arguments)?;
            claim_scm(wallet).await?;
        }
        _ => {
            // Shouldn't happen, but let's be safe.
            return Err(anyhow::anyhow!(
                "Please call the tool with '--help' to see available modes."
            ));
        }
    }
    Ok(())
}
