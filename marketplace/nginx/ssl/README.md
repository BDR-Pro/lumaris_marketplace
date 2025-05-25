# SSL Certificates

Place your SSL certificates in this directory:

- `lumaris.crt` - SSL certificate
- `lumaris.key` - SSL private key

## Generating Self-Signed Certificates for Development

For development purposes, you can generate self-signed certificates using OpenSSL:

```bash
openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout lumaris.key -out lumaris.crt
```

## Production Certificates

For production, use proper SSL certificates from a trusted certificate authority like Let's Encrypt.

Example using Certbot with Let's Encrypt:

```bash
certbot certonly --standalone -d yourdomain.com
```

Then copy the certificates to this directory:

```bash
cp /etc/letsencrypt/live/yourdomain.com/fullchain.pem lumaris.crt
cp /etc/letsencrypt/live/yourdomain.com/privkey.pem lumaris.key
```

