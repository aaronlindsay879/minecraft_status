| env var          | description                                                    | default |
|------------------|----------------------------------------------------------------|---------|
| RUST_LOG         | sets logging level                                             | WARN    |
| SERVER           | server ip/url to check                                         |         |
| PORT             | port to check (will be ignored if applicable SRV record found) | 25565   | 
| REFRESH_INTERVAL | how often to check server status                               | 60s     |
