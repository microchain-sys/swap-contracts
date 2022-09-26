import type { BigNumberish } from 'fuels';
import { bn, NativeAssetId } from 'fuels';

import type { ExchangeContractAbi, TokenContractAbi } from '../../src/types/contracts';

const { TOKEN_AMOUNT, ETH_AMOUNT } = process.env;

export async function initializePool(
  tokenContract: TokenContractAbi,
  exchangeContract: ExchangeContractAbi,
  overrides: { gasPrice: BigNumberish }
) {
  const wallet = tokenContract.wallet!;
  const tokenAmount = bn(TOKEN_AMOUNT || '0x44364C5BB');
  const ethAmount = bn(ETH_AMOUNT || '0xE8F272');
  const address = {
    value: wallet.address.toB256(),
  };
  const tokenId = {
    value: tokenContract.id.toB256(),
  };

  await tokenContract.functions.mint_coins(tokenAmount).txParams(overrides).call();
  await tokenContract.functions
    .transfer_token_to_output(tokenAmount, tokenId, address)
    .txParams({
      ...overrides,
      variableOutputs: 1,
    })
    .call();

  console.log('Balances');
  console.log('ETH', await wallet.getBalance(NativeAssetId));
  console.log('Token', await wallet.getBalance(tokenContract.id.toB256()));

  process.stdout.write('Initialize pool\n');
  const deadline = (await wallet.provider.getBlockNumber()).add(1000);
  await exchangeContract
    .multiCall([
      exchangeContract.functions.deposit().callParams({
        forward: [ethAmount, NativeAssetId],
      }),
      exchangeContract.functions.deposit().callParams({
        forward: [tokenAmount, tokenContract.id.toB256()],
      }),
      exchangeContract.functions.add_liquidity(1, deadline),
    ])
    .txParams({
      ...overrides,
      variableOutputs: 2,
      gasLimit: 100_000_000,
    })
    .call();

  console.log('Pool initialized, running a test swap');

  // Make a test swap

  const chainInfo = await wallet.provider.getChain()

  const tx = await exchangeContract.functions
    .swap_with_minimum(1, chainInfo.latestBlock.height.toNumber() + 100)
    .callParams({
      forward: [100, NativeAssetId],
    })
    .txParams({
      gasPrice: 1,
      variableOutputs: 2,
      gasLimit: 100_000_000,
    })
    .call();
  console.log('swapped', tx);
}
