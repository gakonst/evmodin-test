pragma solidity ^0.8.6;

abstract contract Hevm {
    // sets the block timestamp to x
    function warp(uint x) public virtual;
    // sets the block number to x
    function roll(uint x) public virtual;
    // sets the slot loc of contract c to val
    function store(address c, bytes32 loc, bytes32 val) public virtual;
    function ffi(string[] calldata) external virtual returns (bytes memory);
}

contract Greet {
     Hevm internal constant hevm =
        Hevm(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D);

    uint256 x;

    function setUp() public {
        x = 1;
    }

    function testFoo() public {
        // TODO: This fails right not
        // hevm.roll(1);
        require(x == 1, "not one");
    }

    function testFailFoo() public {
        require(x == 2, "not two");
    }
}
