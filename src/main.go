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

type ServerCert struct {
	CertFilename string
	KeyFilename string
	KeyPassword  string
}

type HomeServerConfig struct {
	DefaultCert ServerCert
}

// TODO::
//   - use ENV variables to set name of paths you are using
//   - choose a good set of cypherSuits to accept
//   - run an security audit and setup proper headers
func loadServerConfig() (*HomeServerConfig, error) {
	return &HomeServerConfig{
		DefaultCert: ServerCert {
			CertFilename: ".tls-certs/jhertz_ink_crt.pem",
			KeyFilename:  ".tls-certs/jhertz_ink_key.pem",
			KeyPassword: "",
		},
	}, nil
}

func (cfg * HomeServerConfig) loadCertificates() ([]tls.Certificate, error) {

	leafCertCfg  := &cfg.DefaultCert
	tlsCert, err := tls.LoadX509KeyPair( leafCertCfg.CertFilename, leafCertCfg.KeyFilename )
	if err != nil {
		return nil, fmt.Errorf("failed to load leaf certiciate key pair: %w", err)
	}

	return []tls.Certificate{ tlsCert }, nil
}

func generateTLSConfig() (*tls.Config, error) {
	homeCfg, error := loadServerConfig()
	if error != nil {
		return nil, error
	}

	certs, err := homeCfg.loadCertificates()
	if err != nil {
		return nil, err
	}

	hostname, err := os.Hostname()
	if err != nil {
		return nil, err
	}

	// https://gist.github.com/denji/12b3a568f092ab951456
	return &tls.Config{
		MinVersion: tls.VersionTLS12,
		MaxVersion: tls.VersionTLS13,
		Certificates: certs,
		// ClientCAs:  pool,
		ServerName: hostname,
	}, nil
}


func setupEndPoints() *http.ServeMux {
	router := http.NewServeMux()
	fs     := http.FileServer(http.Dir(".static-files"))
	router.Handle("GET /", fs)
	// router.HandleFunc("GET /", fs)
// 	router.HandleFunc("GET /{$}", func(w http.ResponseWriter, r *http.Request) {
// 		fmt.Fprintf(w, `
// <title> Welcome Page </title>
// <p> Hi there and welcome to <a href="https://jhertz.ink"> jhertz.ink </a></p>
// `)
// 	})

	return router
}



func main() {
	log.SetFlags(log.Lshortfile)

	cfg, err := generateTLSConfig()
	if err != nil {
		log.Fatal("Error setting TLS config: ", err)
	}

	router := setupEndPoints()
	server := &http.Server{
		Addr:         "0.0.0.0:443",
		Handler:      router,
		TLSConfig:    cfg,
	}

	ln, err := net.Listen("tcp", server.Addr)
	if err != nil {
		log.Fatalf("failed to attach to server addr: %s", err)
	}
	syscall.Setuid(1001)

	defer ln.Close()
	err = server.ServeTLS(ln, "", "")
	if err != nil {
		log.Fatalf("Failed to start server: %v", err)
	}
}

