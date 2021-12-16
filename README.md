## SCM contract

Users can `invest` in the ICO smart contract with WETH. 2 minutes after 1000 WETH have been raised investors can `claim` 10 times more `SCM` tokens than they invested `WETH`.
After an investment increased the total raised WETH over 1000 no client will be able to invest anymore.

#### Installing Dependencies
Requires `npm` to be installed.

```bash
cd contract
npm install
```

#### Running Tests
```bash
cd contract
npx hardhat test
```

#### Running Tests With Coverage
```bash
cd contract
npx hardhat coverage --testfiles "./test/test.ts"
```

The created HTML report can be found here: `/coverage/index.html`.

#### Spinning Up Local Test Network
Start blank test network on localhost.  
This will be a blocking call so keep it running and open another tab for the next command.

```bash
cd contract
npx hardhat node
```

Deploy ICO and some users with a WETH balance on localhost with the second tab.  

```bash
npx hardhat run --network localhost scripts/deploy\_locally.ts
```


## Rust Client to Interact With The ICO

#### Capabilities
```
Scm Client 1.0
Let's you interact with the SCM ICO and token

USAGE:
    scm_client [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    claim-scm        Claim your hard earned SCM
    claimable-scm    Tells you how much SCM you can claim
    help             Prints this message or the help of the given subcommand(s)
    ico-status       Queries status of ICO.
    invest           Invest in the ICO
    scm-balance      Tells you how much SCM you own
    weth-balance     Tells you how much WETH you own
```

#### Building the Client
Requires `cargo` to be installd.

```bash
cd scm_client
cargo build
```

#### Running the Client
This will also build the client first if you didn't already.  
This will print the help output and tell you about other commands it supports.

```bash
cd scm_client
cargo run -- --help
```
