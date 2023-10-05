# minecraft_status
Simple webserver to query and display the status of one or more given minecraft servers. 
Shows status of all servers on main page, and individual statuses on paths given by the server ip - such as `status.docker.localhost/your.server.ip`

![image demo](docs/img.png)

## Unsafe code usage
Unsafe code is denied in both the `dns` and `minecraft_status` crates, with an exception for finding DNS servers on windows as that relies on calling [GetAdaptersAddresses](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses) and processing the resulting [IP_ADAPTER_ADDRESSES_LH](https://learn.microsoft.com/en-us/windows/win32/api/iptypes/ns-iptypes-ip_adapter_addresses_lh).


## Env vars

| env var          | description                                                                                                                     | default | example                          |
|------------------|---------------------------------------------------------------------------------------------------------------------------------|---------|----------------------------------|
| RUST_LOG         | sets logging level                                                                                                              | WARN    | DEBUG                            |
| PORT             | port for server to listen on                                                                                                    | 3000    | 80                               |
| SERVER           | server ip/url to check, in format [ip/url]:[port] (where :[port] is optional). multiple can be passed by separating with commas |         | your.server,your.other.server:40 |
| REFRESH_INTERVAL | how often to check server status                                                                                                | 60s     | 5m                               |

## Usage with docker compose 
```yaml
  minecraft_status:
    image: svvqqrdoiofnohfchkjcdgoixnnsta/minecraft_status
    container_name: minecraft_status
    ports:
      - "80:80"
    environment:
      SERVER: [YOUR SERVER]
      REFRESH_INTERVAL: 5m
      PORT: 80
```

## Usage with docker compose + traefik
```yaml
  minecraft_status:
    image: svvqqrdoiofnohfchkjcdgoixnnsta/minecraft_status
    container_name: minecraft_status
    environment:
      SERVER: [YOUR SERVER]
      REFRESH_INTERVAL: 5m
    labels:
      traefik.enable: true
      traefik.http.routers.status.rule: Host(`status.docker.localhost`)
      traefik.http.services.status.loadbalancer.server.port: 3000
      traefik.http.routers.status.entrypoints: web
```
