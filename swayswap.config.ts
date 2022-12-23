import dotenv from 'dotenv';
import { createConfig, replaceEventOnEnv } from 'swayswap-scripts';

const { NODE_ENV, OUTPUT_ENV } = process.env;

function getEnvName() {
  return NODE_ENV === 'test' ? '.env.test' : '.env';
}

dotenv.config({
  path: `./docker/${getEnvName()}`,
});

const getDeployOptions = ({ salt }: { salt?: number } = {}) => ({
  gasPrice: Number(process.env.GAS_PRICE || 0),
  salt: '0x' + (salt || 0).toString(16).padStart(64, '0'),
});

// So the addresses change each deploy
const saltBase = Math.floor(Date.now() / 10000);

export default createConfig({
  types: {
    artifacts: './packages/contracts/**/out/debug/**-abi.json',
    output: './packages/app/src/types/contracts',
  },
  contracts: [
    {
      name: 'REGISTRY_CONTRACT_ID',
      path: './packages/contracts/registry_contract',
      options: getDeployOptions({ salt: saltBase }),
    },
    {
      name: 'ROUTER_CONTRACT_ID',
      path: './packages/contracts/router_contract',
      options: getDeployOptions(),
    },
    {
      name: 'VAULT_CONTRACT_ID',
      path: './packages/contracts/vault_contract',
      options: getDeployOptions(),
    },
    {
      name: 'VITE_TOKEN_1_ID',
      path: './packages/contracts/token_contract',
      options: getDeployOptions({ salt: 1 + saltBase }),
    },
    {
      name: 'VITE_TOKEN_2_ID',
      path: './packages/contracts/token_contract',
      options: getDeployOptions({ salt: 2 + saltBase }),
    },
    {
      name: 'VITE_EXCHANGE_1_ID',
      path: './packages/contracts/exchange_contract',
      options: (contracts) => {
        const contractDeployed = contracts.find((c) => c.name === 'VITE_TOKEN_1_ID')!;
        return {
          ...getDeployOptions({ salt: 1 + saltBase }),
          storageSlots: [
            {
              key: '0x0000000000000000000000000000000000000000000000000000000000000001',
              value: contractDeployed.contractId,
            },
          ],
        };
      },
    },
    {
      name: 'VITE_EXCHANGE_2_ID',
      path: './packages/contracts/exchange_contract',
      options: (contracts) => {
        const contractDeployed = contracts.find((c) => c.name === 'VITE_TOKEN_2_ID')!;
        return {
          ...getDeployOptions({ salt: 2 + saltBase }),
          storageSlots: [
            {
              key: '0x0000000000000000000000000000000000000000000000000000000000000001',
              value: contractDeployed.contractId,
            },
          ],
        };
      },
    },
  ],
  onSuccess: (event) => {
    replaceEventOnEnv(`./packages/app/${OUTPUT_ENV || getEnvName()}`, event);
  },
});
