import { ethers, waffle } from "hardhat";
import { SignerWithAddress } from "@nomiclabs/hardhat-ethers/signers"
import { Contract } from "ethers";
import WETH9 from "canonical-weth/build/contracts/WETH9.json";

const ETHER = ethers.utils.parseEther("1");

async function deployIco(scammer: SignerWithAddress, acceptedToken: Contract) {
    const ICO = await ethers.getContractFactory("ICO", scammer);
    const ico = await ICO.deploy(acceptedToken.address);
    return await ico.deployed();
}

async function deployWeth(wethOwner: SignerWithAddress) {
    return await waffle.deployContract(wethOwner, WETH9, []);
}

async function getScmContract(ico: Contract) {
    const scmAddress = await ico.scmToken();
    const SCM = await ethers.getContractFactory("SCM");
    return SCM.attach(scmAddress);
}

async function initIco(scammer: SignerWithAddress, wethOwner: SignerWithAddress) {
    const weth = await deployWeth(wethOwner);
    const ico = await deployIco(scammer, weth);
    const scm = await getScmContract(ico);
    return [ico, weth, scm];
}

async function deploy_locally() {
    const [scammer, user1, wethOwner, user2, user3] = await ethers.getSigners();
    const [ico, weth, scm] = await initIco(scammer, wethOwner);

    await weth.connect(scammer).deposit({value: ETHER.mul(1000)});
    await weth.connect(user1).deposit({value: ETHER.mul(1000)});
    await weth.connect(user2).deposit({value: ETHER.mul(1000)});
    await weth.connect(user3).deposit({value: ETHER.mul(1000)});

    console.log("ico address:", ico.address);
    console.log("scm address:", scm.address);
    console.log("weth address:", weth.address);
    console.log("scammer address:", scammer.address);
    console.log("user address:", user1.address);
}

deploy_locally();
