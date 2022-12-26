/* Autogenerated file. Do not edit manually. */

/* tslint:disable */
/* eslint-disable */

/*
  Fuels version: 0.28.0
  Forc version: 0.32.2
  Fuel-Core version: 0.15.1
*/

import { Interface, Contract } from 'fuels';
import type { Provider, BaseWalletLocked, AbstractAddress } from 'fuels';
import type { VaultContractAbi, VaultContractAbiInterface } from '../VaultContractAbi';

const _abi = {
  types: [
    {
      typeId: 0,
      type: '()',
      components: [],
      typeParameters: null,
    },
    {
      typeId: 1,
      type: 'b256',
      components: null,
      typeParameters: null,
    },
    {
      typeId: 2,
      type: 'enum Error',
      components: [
        {
          name: 'MustBeCalledByOwner',
          type: 0,
          typeArguments: null,
        },
      ],
      typeParameters: null,
    },
    {
      typeId: 3,
      type: 'struct VaultFee',
      components: [
        {
          name: 'start_time',
          type: 5,
          typeArguments: null,
        },
        {
          name: 'start_fee',
          type: 4,
          typeArguments: null,
        },
        {
          name: 'current_fee',
          type: 4,
          typeArguments: null,
        },
        {
          name: 'change_rate',
          type: 4,
          typeArguments: null,
        },
      ],
      typeParameters: null,
    },
    {
      typeId: 4,
      type: 'u16',
      components: null,
      typeParameters: null,
    },
    {
      typeId: 5,
      type: 'u32',
      components: null,
      typeParameters: null,
    },
  ],
  functions: [
    {
      inputs: [
        {
          name: 'pool',
          type: 1,
          typeArguments: null,
        },
      ],
      name: 'claim_fees',
      output: {
        name: '',
        type: 0,
        typeArguments: null,
      },
    },
    {
      inputs: [],
      name: 'get_fees',
      output: {
        name: '',
        type: 3,
        typeArguments: null,
      },
    },
    {
      inputs: [
        {
          name: 'start_fee',
          type: 4,
          typeArguments: null,
        },
        {
          name: 'change_rate',
          type: 4,
          typeArguments: null,
        },
      ],
      name: 'set_fees',
      output: {
        name: '',
        type: 0,
        typeArguments: null,
      },
    },
  ],
  loggedTypes: [
    {
      logId: 0,
      loggedType: {
        name: '',
        type: 2,
        typeArguments: [],
      },
    },
  ],
  messagesTypes: [],
};

export class VaultContractAbi__factory {
  static readonly abi = _abi;
  static createInterface(): VaultContractAbiInterface {
    return new Interface(_abi) as unknown as VaultContractAbiInterface;
  }
  static connect(
    id: string | AbstractAddress,
    walletOrProvider: BaseWalletLocked | Provider
  ): VaultContractAbi {
    return new Contract(id, _abi, walletOrProvider) as unknown as VaultContractAbi;
  }
}
