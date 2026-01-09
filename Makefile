# dirs
BIN_DIR := bin
SRC_DIR := src

EXE := $(BIN_DIR)/server

all: $(EXE)

$(EXE): $(SRC) | $(BIN_DIR)
	GOOS=linux GOARCH=arm64 go -C $(SRC_DIR) build -o $(PWD)/$@

$(BIN_DIR):
	mkdir -p $@

.PHONNY: clean test push
clean:
	@rm -rfv $(BIN_DIR)

test:
	HOME_TEST=1 go run $(SRC_DIR)/main.go

push: $(EXE)
	ssh home "sudo systemctl stop server.service"
	rsync bin/server home_server:~/server
	rsync -av ./frontend/ home_server:~/.static-files
	ssh home "sudo systemctl start server.service"

