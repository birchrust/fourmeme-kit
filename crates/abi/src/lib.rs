use alloy::sol;

sol! {
    #[sol(rpc)]
    FourMemeContract,
    "src/fourmeme_abi.json"
}

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address) external view returns (uint256);
        function approve(address spender, uint256 allowance) external;
        function allowance(address, address) external view returns (uint256);
    }
}

sol! {
    function swapExactETHForTokens(
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external payable;

}
