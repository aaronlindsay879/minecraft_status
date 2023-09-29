| env var          | description                                                    | default |
|------------------|----------------------------------------------------------------|---------|
| RUST_LOG         | sets logging level                                             | WARN    |
| PORT             | port for server to listen on                                   | 3000    |
| SERVER           | server ip/url to check                                         |         |
| SERVER_PORT      | port to check (will be ignored if applicable SRV record found) | 25565   | 
| REFRESH_INTERVAL | how often to check server status                               | 60s     |
