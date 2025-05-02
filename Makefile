# dirs
BIN_DIR := bin

# files c:
SRC := $(wildcard src/*.go)
EXE := $(BIN_DIR)/server

all: $(EXE)

$(EXE): $(SRC) | $(BIN_DIR)
	GOARCH=arm64 go build -o $@ $<

$(BIN_DIR):
	mkdir -p $@

.PHONNY: clean
clean:
	@rm -rfv $(BIN_DIR)
