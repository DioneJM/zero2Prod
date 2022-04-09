# zero2prod

## Development
1. Run `./scripts/init_db.sh`
   - Starts the postgres db
2. Run `./scripts/init_redis.sh`
   - Starts the redis db
3. Run `cargo run`

### Things to consider
- When restarting the services ensure that there are no images for the postgres and redis container being run on docker
