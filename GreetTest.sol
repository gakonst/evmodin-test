pragma solidity ^0.8.6;

contract Greet {
    uint256 x;

    function setUp() public {
        x = 1;
    }

    function testFoo() public {
        require(x == 1, "not one");
    }

    function testFailFoo() public {
        require(x == 2, "not two");
    }
}
