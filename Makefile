bold := $(shell tput bold)
sgr0 := $(shell tput sgr0)

build:
	@echo "$(bold)Compiling the server$(sgr0)"
	@cargo build
	@echo "$(bold)Compiling the wasm...$(sgr0)"
	@wasm-pack build src/weblib/ --target web -d ${PWD}/static
	@printf "\n!index.html\n!.gitignore\n" >> ${PWD}/static/.gitignore


build-release:
	@echo "$(bold)Compiling the optimized server$(sgr0)"
	@cargo build --release
	@echo "$(bold)Compiling the optimnized wasm...$(sgr0)"
	@wasm-pack build src/weblib/ --release --target web -d ${PWD}/static
	@printf "\n!index.html\n!.gitignore\n" >> ${PWD}/static/.gitignore

run:
	cargo run bin --server