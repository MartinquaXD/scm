/**
 * @type import('hardhat/config').HardhatUserConfig
 */
require("solidity-coverage");
const contract = require('truffle-contract');
const wethArtifact = require('canonical-weth');
import "@nomiclabs/hardhat-waffle";

const weth = contract(wethArtifact);

module.exports = {
  solidity: "0.8.6",
  weth,
  networks: {
    hardhat: {
        mining: {
        auto: true,
        interval: 1000
        }
    },
  }
};
