PROJECT_NAME := llm-cost-exporter
CARGO := cargo
DOCKER := docker

build:
	$(CARGO) build --release

docker-build:
	$(DOCKER) build -t $(PROJECT_NAME) .

clean:
	$(CARGO) clean

run:
	$(CARGO) run

metrics:
	curl localhost:8000/metrics

ci: lint test

lint:
	$(CARGO) clippy -- -D warnings

test:
	$(CARGO) test -- --nocapture
