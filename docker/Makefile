services-run:
	docker compose --env-file ./.env -p swayswap_local up --build -d

services-clean:
	docker compose -p swayswap_local down --rmi local -v --remove-orphans

services-run-test:
	docker compose --env-file ./.env.test -p swayswap_test up --build -d

services-clean-test:
	docker compose -p swayswap_test down --rmi local -v --remove-orphans
