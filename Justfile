set dotenv-load := true
set export := true
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

export DEPLOY_SERVER := "ldev@192.168.1.69"

alias b := build
alias r := run
alias prep := sqlx-prepare

run:
    docker compose up --watch

build:
    docker compose build
    docker compose up --watch

check: check_rust check_ts

check_rust: db-up
    cargo check

[working-directory("smppgc/client")]
check_ts:
    tsc

db-up:
    docker compose up --detach db

sqlx-prepare:
    cargo sqlx prepare --workspace

    docker compose up --watch

[working-directory('smppgc')]
sqlx-reset-db: db-up
    cargo sqlx database reset

deploy:
    diststar
    cat .diststar-out/smppgc/gnu+linux_arm_v7/smppgc.docker | ssh $DEPLOY_SERVER docker load
    ssh $DEPLOY_SERVER ~/source/repos/ldeveuorg-infra/deploy.sh

promote_beta_to_prod:
    ssh $DEPLOY_SERVER ~/source/repos/ldeveuorg-infra/smpp/beta2prod.sh
