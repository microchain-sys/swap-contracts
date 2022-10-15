import { bn } from 'fuels';
import type { TokenContractAbi } from '../../src/types/contracts';

const { MINT_AMOUNT } = process.env;

export async function initializeTokenContract(
  tokenContract: TokenContractAbi,
  overrides: any
) {
  const mintAmount = bn(MINT_AMOUNT || '2000000000000');
  const address = {
    value: tokenContract.wallet!.address.toB256(),
  };

  const { value: owner } = await tokenContract.functions.get_owner().get()
  if (owner.value !== '0x0000000000000000000000000000000000000000000000000000000000000000') {
    return;
  }

  process.stdout.write('Initialize Token Contract\n');
  await tokenContract.functions.initialize(mintAmount, address).txParams(overrides).call();
}
