/* Autogenerated file. Do not edit manually. */

/* tslint:disable */
/* eslint-disable */

/*
  Fuels version: 0.28.0
  Forc version: 0.32.2
  Fuel-Core version: 0.15.1
*/

import type {
  Interface,
  FunctionFragment,
  DecodedValue,
  Contract,
  BytesLike,
  BigNumberish,
  InvokeFunction,
  BN,
} from 'fuels';

import type { Option, Enum } from './common';

export type ErrorInput = Enum<{
  UnorderedTokens: [];
  AlreadyInitialized: [];
  AlreadyRegistered: [];
  InvalidContractCode: [];
  PoolInitialized: [];
}>;
export type ErrorOutput = ErrorInput;

interface RegistryContractAbiInterface extends Interface {
  functions: {
    add_exchange_contract: FunctionFragment;
    exchange_contract_root: FunctionFragment;
    get_exchange_contract: FunctionFragment;
    initialize: FunctionFragment;
    is_pool: FunctionFragment;
  };

  encodeFunctionData(functionFragment: 'add_exchange_contract', values: [string]): Uint8Array;
  encodeFunctionData(functionFragment: 'exchange_contract_root', values: []): Uint8Array;
  encodeFunctionData(
    functionFragment: 'get_exchange_contract',
    values: [string, string]
  ): Uint8Array;
  encodeFunctionData(functionFragment: 'initialize', values: [string]): Uint8Array;
  encodeFunctionData(functionFragment: 'is_pool', values: [string]): Uint8Array;

  decodeFunctionData(functionFragment: 'add_exchange_contract', data: BytesLike): DecodedValue;
  decodeFunctionData(functionFragment: 'exchange_contract_root', data: BytesLike): DecodedValue;
  decodeFunctionData(functionFragment: 'get_exchange_contract', data: BytesLike): DecodedValue;
  decodeFunctionData(functionFragment: 'initialize', data: BytesLike): DecodedValue;
  decodeFunctionData(functionFragment: 'is_pool', data: BytesLike): DecodedValue;
}

export class RegistryContractAbi extends Contract {
  interface: RegistryContractAbiInterface;
  functions: {
    add_exchange_contract: InvokeFunction<[exchange_id: string], void>;
    exchange_contract_root: InvokeFunction<[], string>;
    get_exchange_contract: InvokeFunction<[token_a: string, token_b: string], Option<string>>;
    initialize: InvokeFunction<[template_exchange_id: string], void>;
    is_pool: InvokeFunction<[addr: string], boolean>;
  };
}
