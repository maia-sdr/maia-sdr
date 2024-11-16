all: key.pem cert.pem

key.pem cert.pem: Makefile
	openssl req -new -newkey rsa:4096 -days 10000 -nodes -x509 \
	    -subj "/C=ES/ST=Madrid/L=Madrid/O=Maia SDR/OU=maia-httpd/CN=*" \
	    -keyout key.pem  -out cert.pem

clean:
	rm -f key.pem cert.pem

.PHONY: all clean
