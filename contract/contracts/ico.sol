// contracts/ico.sol
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract SCM is ERC20 {
    address private owner;

    constructor() ERC20("Scam", "SCM") {
        owner = msg.sender;
    }

    function mint(uint amount) external {
        require(msg.sender == owner, "Only the owner is allowed to mint tokens.");
        _mint(msg.sender, amount);
    }
}

contract ICO {
    ERC20 immutable private acceptedToken;

    // how much WETH has to be invested before ICO
    uint constant private WETH_LIMIT = 1000 ether;

    // seconds which need to pass after ICO until users can
    // claim SCM token
    uint constant private CLAIM_DELAY = 2 minutes;

    address payable immutable private myWallet;

    // track how much WETH each investor sent
    mapping(address => uint) private investors;

    // earliest timestamp when the investors can claim the token
    uint private claimableAt = 0;

    uint private raisedWeth = 0;

    SCM immutable public scmToken;

    constructor(ERC20 token) {
        // every WETH people send, shall be added to my private wallet
        myWallet = payable(msg.sender);
        acceptedToken = token;
        scmToken = new SCM();
    }

    function claim() external {
        require(isCompleted(), "The ICO has to be completed first.");
        scmToken.transfer(msg.sender, investors[msg.sender]);
        investors[msg.sender] = 0;
    }

    function isCompleted() public view returns(bool) {
        return claimableAt != 0 && block.timestamp >= claimableAt;
    }

    function claimableScm(address investor) external view returns (uint256) {
        return investors[investor];
    }

    function invest(uint amount) external {
        require(claimableAt == 0, "The investment limit has already been reached.");
        require(acceptedToken.allowance(msg.sender, address(this)) >= amount, "Our WETH allowance is not big enough.");
        require(acceptedToken.transferFrom(msg.sender, myWallet, amount), "Not enough funds.");
        investors[msg.sender] += 10 * amount;

        raisedWeth += amount;
        if(raisedWeth >= WETH_LIMIT) {
            claimableAt = block.timestamp + CLAIM_DELAY;
            scmToken.mint(10 * raisedWeth);
        }
    }
}
