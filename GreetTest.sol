pragma solidity ^0.8.6;

contract Greet {
    uint256 x;

    function setUp() public {
        x = 1;
    }

    function testFoo() public {

    }

    function testBar() public {
        revert("1111");
    }
}
