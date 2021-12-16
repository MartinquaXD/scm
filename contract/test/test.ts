import { expect } from "chai";
import { ethers, waffle } from "hardhat";
import { SignerWithAddress } from "@nomiclabs/hardhat-ethers/signers"
import { Contract, BigNumberish } from "ethers";
import WETH9 from "canonical-weth/build/contracts/WETH9.json";

const ICO_DELAY = 60 * 2;
const SCM_PER_WETH = 10;

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

async function investInIco(user: SignerWithAddress, weth: Contract, ico: Contract, amount: BigNumberish) {
    await weth.connect(user).deposit({value: amount});
    await weth.connect(user).approve(ico.address, amount);
    expect(await ico.connect(user).invest(amount));
}

async function getTime() {
    const blockNumBefore = await ethers.provider.getBlockNumber();
    const blockBefore = await ethers.provider.getBlock(blockNumBefore);
    return new Date(blockBefore.timestamp * 1000);
}

async function progressTime(seconds: number) {
    await ethers.provider.send("evm_increaseTime", [seconds]);
    await ethers.provider.send("evm_mine", []);
}

async function expectBalance(erc20: Contract, user: SignerWithAddress, amount: BigNumberish) {
    const balance = await erc20.balanceOf(user.address);
    expect(balance.eq(amount)).to.be.ok;
}

async function expectClaimableBalance(ico: Contract, user: SignerWithAddress, amount: BigNumberish) {
    const balance = await ico.claimableScm(user.address);
    expect(balance.eq(amount)).to.be.ok;
}

const ETHER = ethers.utils.parseEther("1");
describe("SCM", function () {
    it("allows only ICO contract to mint", async () => {
        const [scammer, user1, wethOwner] = await ethers.getSigners();
        const [ico, weth, scm] = await initIco(scammer, wethOwner);
        await expect(scm.connect(scammer).mint(1)).to.be.reverted;
    });
});

describe("ICO", function () {
    it("respects allowance", async () => {
        const [scammer, user1, wethOwner] = await ethers.getSigners();
        const [ico, weth] = await initIco(scammer, wethOwner);

        // allowance and amount are equal
        await investInIco(user1, weth, ico, ETHER);

        // allowance is bigger than amount, this will only use amount
        // to invest
        await weth.connect(user1).deposit({value: ETHER.mul(2)});
        await weth.connect(user1).approve(ico.address, ETHER.mul(2));
        expect(await ico.connect(user1).invest(ETHER)).to.be.ok;
        await expectBalance(weth, user1, ETHER);

        // allowance is smaller than amount
        await weth.connect(user1).deposit({value: ETHER.mul(2)});
        await weth.connect(user1).approve(ico.address, ETHER);
        await expect(ico.connect(user1).invest(ETHER.mul(2))).to.be.reverted;

        // amount and allowance is bigger than current balance
        await weth.connect(user1).approve(ico.address, ETHER.mul(100));
        await expect(ico.connect(user1).invest(ETHER.mul(100))).to.be.reverted;
    });

    it("stops accepting investments after 1000 WETH", async () => {
        const [scammer, user1, wethOwner] = await ethers.getSigners();
        const [ico, weth] = await initIco(scammer, wethOwner);

        // invest 1000 ETH - 1 wei
        const maxValue = ETHER.mul(1000).sub(1);
        await investInIco(user1, weth, ico, maxValue);

        // final 1 wei investment is allowed
        await investInIco(user1, weth, ico, 1);

        // 1000 WETH reached; not a single wei can be invested anymore
        await weth.connect(user1).deposit({value: 1});
        await weth.connect(user1).approve(ico.address, 1);
        await expect(ico.connect(user1).invest(1)).to.be.reverted;
    });

    it("allows claiming SCM 2 minutes after the ICO", async () => {
        const [scammer, user1, wethOwner] = await ethers.getSigners();
        const [ico, weth] = await initIco(scammer, wethOwner);
        await investInIco(user1, weth, ico, ETHER.mul(1000));
        await progressTime(ICO_DELAY - 5);
        await expect(ico.connect(user1).claim()).to.be.reverted;
        await progressTime(10);
        expect(await ico.connect(user1).claim()).to.be.ok;
    });

    it("will give investors 10 SCM per invested WETH", async () => {
        const [scammer, user1, user2, user3, wethOwner] = await ethers.getSigners();
        const [ico, weth, scm] = await initIco(scammer, wethOwner);

        // investing doesn't give SCM immediately
        const user1Amount = ETHER.mul(300);
        await investInIco(user1, weth, ico, user1Amount);
        await expectBalance(scm, user1, 0);

        const user2Amount = ETHER.mul(700);
        await investInIco(user2, weth, ico, user2Amount);
        await expectBalance(scm, user2, 0);

        await expectBalance(scm, user3, 0);

        await progressTime(ICO_DELAY);

        // the ICO completing doesn't give SCM
        await expectBalance(scm, user1, 0);
        await expectBalance(scm, user2, 0);
        await expectBalance(scm, user3, 0);


        // only claiming gives SCM
        expect(await ico.connect(user1).claim()).to.be.ok;
        await expectBalance(scm, user1, user1Amount.mul(SCM_PER_WETH));

        expect(await ico.connect(user2).claim()).to.be.ok;
        await expectBalance(scm, user2, user2Amount.mul(SCM_PER_WETH));

        expect(await ico.connect(user3).claim()).to.be.ok;
        await expectBalance(scm, user3, 0);
    });

    it("sends money to scammer immediately", async () => {
        const [scammer, user1, wethOwner] = await ethers.getSigners();
        const [ico, weth] = await initIco(scammer, wethOwner);
        await investInIco(user1, weth, ico, ETHER.mul(1));
        await expectBalance(weth, scammer, ETHER);
    });

    it("allows investors to see their claimable tokens", async () => {
        const [scammer, user1, user2, wethOwner] = await ethers.getSigners();
        const [ico, weth, scm] = await initIco(scammer, wethOwner);

        // investing doesn't give SCM immediately
        const user1Amount = ETHER.mul(300);
        await investInIco(user1, weth, ico, user1Amount);
        await expectClaimableBalance(ico, user1, user1Amount.mul(SCM_PER_WETH));

        const user2Amount = ETHER.mul(700);
        await investInIco(user2, weth, ico, user2Amount);
        await expectClaimableBalance(ico, user2, user2Amount.mul(SCM_PER_WETH));
    });
});

