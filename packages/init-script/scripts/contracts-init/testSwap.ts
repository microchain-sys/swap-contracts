import { bn, NativeAssetId } from 'fuels';

import type { ExchangeContractAbi, RouterContractAbi, TokenContractAbi } from '../../contracts';

const { TOKEN_AMOUNT, ETH_AMOUNT } = process.env;

export async function testSwap(
  routerContract: RouterContractAbi,
  tokenContract: TokenContractAbi,
  exchangeContract: ExchangeContractAbi,
  overrides: any
) {

  console.log('Running test swap');

  // const result = await routerContract.functions.null(
  const result = await routerContract.functions.swap_exact_input(
        exchangeContract.id.toB256(),
        0,
        { Address: { value: wallet.address.toHexString() } },
      )
      .callParams({
        forward: [10, NativeAssetId],
        gasLimit: 10_000_000,
      })
      .addContracts([exchangeContract.id])
      .txParams({
        variableOutputs: 2,
        gasLimit: 100_000_000,
        gasPrice: 1,
      })
      .call();
  console.log(result)
}
