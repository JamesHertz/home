package main

import (
	"crypto/tls"
	"net"
	"syscall"

	// "syscall"
	// "time"
	"fmt"
	"log"
	"net/http"
	"os"
)
// type ServerCert struct {
// }

type HomeServerConfig struct {
	TLSConfig * tls.Config
	// TestingMode bool
	ServerAddr string
	RootDir string
}

// TODO::
//	 - start getting this info from a config file
//   - choose a good set of cypherSuits to accept
//   - run an security audit and setup proper headers
func loadServerConfig() (*HomeServerConfig, error) {
	cfg := &HomeServerConfig{
		TLSConfig: nil,
	}

	if os.Getenv("HOME_TEST") != "" { // in testing mode
		cfg.ServerAddr = "localhost:8080"
		cfg.RootDir    = "frontend"
	} else {
		cfg.ServerAddr = "0.0.0.0:443"
		cfg.RootDir    = ".static-files"
		tls, err      := loadTLSConfig(
			".tls-certs/jhertz_ink_crt.pem",
			".tls-certs/jhertz_ink_key.pem",
		)
		if err != nil {
			return nil, fmt.Errorf("Error setting TLS config: %w", err)
		}
		cfg.TLSConfig = tls
	}

	return cfg, nil
}

func loadTLSConfig(certFilename string, keyFilename string) (*tls.Config, error) {
	cert, err := tls.LoadX509KeyPair( 
		certFilename, keyFilename,
	)

	if err != nil {
		return nil, fmt.Errorf("failed to load leaf certiciate key pair: %w", err)
	}

	hostname, err := os.Hostname()
	if err != nil {
		return nil, err
	}

	// https://gist.github.com/denji/12b3a568f092ab951456
	return &tls.Config{
		MinVersion: tls.VersionTLS12,
		MaxVersion: tls.VersionTLS13,
		Certificates: []tls.Certificate{ cert },
		ServerName: hostname,
	}, nil
}

func setupEndPoints(cfg * HomeServerConfig) *http.ServeMux {
	router := http.NewServeMux()
	fs     := http.FileServer(http.Dir(cfg.RootDir))
	router.Handle("GET /", fs)
	return router
}

func main() {
	log.SetFlags(log.Lshortfile)

	cfg, err := loadServerConfig()
	if err != nil {
		log.Fatal("Error loading home config: ", err)
	}

	server := &http.Server{
		Addr:         cfg.ServerAddr,
		TLSConfig:    cfg.TLSConfig,
		Handler:      setupEndPoints(cfg), // TODO: what do you want to do with this
	}

	ln, err := net.Listen("tcp", server.Addr)
	if err != nil {
		log.Fatalf("failed to attach to server addr: %s", err)
	}

	defer ln.Close()
	if server.TLSConfig == nil {
		log.Printf("starting server at http://%s", cfg.ServerAddr)
		err = server.Serve(ln)
	} else {
		syscall.Setuid(1001)
		err = server.ServeTLS(ln, "", "")
	}

	if err != nil {
		log.Fatalf("Failed to start server: %v", err)
	}
}
