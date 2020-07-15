bold := $(shell tput bold)
sgr0 := $(shell tput sgr0)
# File to put in the generated gitignore
STATIC_FILES =	""			\
				"!index.js"	\
				"!.gitignore"\
				"!index.html"\
				"!app.css"

build:
	@echo "$(bold)Compiling the server$(sgr0)"
	@cargo build
	@echo "$(bold)Compiling the wasm...$(sgr0)"
	@wasm-pack build src/weblib/ --target web -d ${PWD}/static
	@echo "$(STATIC_FILES)" | tr " " "\n" >> ${PWD}/static/.gitignore


build-release:
	@echo "$(bold)Compiling the optimized server$(sgr0)"
	@cargo build --release
	@echo "$(bold)Compiling the optimnized wasm...$(sgr0)"
	@wasm-pack build src/weblib/ --release --target web -d ${PWD}/static
	@echo "$(STATIC_FILES)" | tr " " "\n" >> ${PWD}/static/.gitignore

run:
	cargo run bin --server

test:
	@cargo test
	@wasm-pack test src/weblib/ --node