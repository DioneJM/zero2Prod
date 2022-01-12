# Using the latest rust stable release
FROM rust:1.56.0

# Go into directory `app`
#(or create directory if it doesn't exist then enter it)
WORKDIR /app

# Copy all files working environment to Docker image
COPY . .

# Build the release binary
RUN cargo build --release

# Execute the binary
# This gets called when `docker run` is executed
ENTRYPOINT ["./target/release/zero2prod"]