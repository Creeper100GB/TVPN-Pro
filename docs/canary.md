# Canary-Endpunkt

Ein kleiner Server, der jede Verbindung protokolliert: Quell-IP, Port, Protokoll, Zeit. Damit lässt sich in der Test-VM beweisen, ob Pakete außerhalb des Tunnels ankommen. Er ist die Gegenstelle für die Leak-Matrix (`PLAN.md` Abschnitt 8).

## Was er zeigt und was nicht

**Zeigt:** ob und mit welcher Quell-IP ein Paket den Rechner verlassen hat, über welches Protokoll, zu welcher Zeit.

**Zeigt nicht:** was eine App sonst noch tut, ob eine Verbindung über einen anderen Weg lief, der den Canary nicht erreicht, und ob ein Ziel, das nicht der Canary ist, geleakt wurde. Ein Leak zu einem fremden Server bleibt unsichtbar.

## Aufbau

- Ein VPS mit öffentlicher IPv4 **und** IPv6 (sonst sind IPv6-Leaks unsichtbar).
- Linux, systemd, SSH nur mit Schlüssel.
- Domain optional. Mit Domain gibt es Let's-Encrypt-TLS; ohne Domain reicht ein selbstsigniertes Zertifikat und `curl -k`.
- Kein Reverse-Proxy. Der Proxy würde die Quell-IP verdecken; der Dienst muss den direkten Peer sehen.

### Variante A: nur mitschneiden (5 Minuten)

Auf dem Server:
```bash
sudo apt install -y tcpdump
sudo tcpdump -n -i any 'not port 22' -l | tee -a /var/log/tvpn-canary.log
```
Das zeigt jede Verbindung mit Quell-IP, Ziel-Port und Protokoll. Nach dem Test Strg+C, Log prüfen, Log löschen. Nachteil: man muss selbst lesen und es ist nicht automatisierbar.

### Variante B: Echo-Dienst (empfohlen)

Zwei kleine Python-Dienste, keine Abhängigkeiten außer der Standardbibliothek: `tests/canary/canary.py`.

```bash
# auf dem Server
sudo useradd --system --home /var/lib/canary --shell /usr/sbin/nologin canary
sudo mkdir -p /opt/canary /var/log/canary /etc/canary
sudo cp canary.py /opt/canary/canary.py
sudo chown -R canary:canary /opt/canary /var/log/canary /etc/canary
```

HTTPS-Echo (Port 443, Zertifikat nötig):
```bash
sudo -u canary python3 /opt/canary/canary.py https --port 443 \
    --cert /etc/canary/fullchain.pem --key /etc/canary/privkey.pem
curl https://canary.example.net/echo
# {"seen_ip":"…","seen_port":51234,"time":"2026-11-21T10:00:00+00:00"}
```

UDP-Mitschnitt (Port 9999, fängt QUIC/WebRTC/UDP):
```bash
sudo -u canary python3 /opt/canary/canary.py udp --port 9999
```

DNS-Mitschnitt (optional, nur mit eigener Domain): `dnsmasq` als autoritativer Server für eine Subdomain, siehe unten.

### systemd

`/etc/systemd/system/canary-https.service`:
```ini
[Unit]
Description=TVPN canary HTTPS echo
After=network-online.target
Wants=network-online.target

[Service]
User=canary
Group=canary
ExecStart=/usr/bin/python3 /opt/canary/canary.py https --port 443 --cert /etc/canary/fullchain.pem --key /etc/canary/privkey.pem
AmbientCapabilities=CAP_NET_BIND_SERVICE
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths=/var/log/canary
RestrictAddressFamilies=AF_INET AF_INET6
StandardOutput=append:/var/log/canary/https.log
StandardError=append:/var/log/canary/https.log
Restart=always

[Install]
WantedBy=multi-user.target
```

`canary-udp.service` ist identisch, nur `ExecStart=… canary.py udp --port 9999` und ohne `AmbientCapabilities` (Port über 1024).

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now canary-https canary-udp
```

### Firewall auf dem Server

Nur das Nötige:
```bash
sudo ufw default deny incoming
sudo ufw allow 22/tcp
sudo ufw allow 443/tcp
sudo ufw allow 9999/udp
sudo ufw enable
```
Port 80 nur, wenn Let's Encrypt über HTTP-01 erneuert.

### Log-Rotation

`/etc/logrotate.d/tvpn-canary`:
```
/var/log/canary/*.log {
    daily
    rotate 7
    compress
    missingok
    notifempty
    copytruncate
}
```

## Datenschutz-Regeln für den Canary

- Geloggt werden **nur**: Zeit (UTC), Quell-IP, Quell-Port, Protokoll, Byte-Zahl bei UDP.
- **Nicht** geloggt werden: Pfad, Query-String, User-Agent, Referer, Cookies, Nutzdaten.
- Aufbewahrung 7 Tage, danach löschen. Nach jeder Testreihe das Log leeren.
- Zugriff nur per SSH-Schlüssel, kein Web-UI, keine Analytics.
- Der Canary ist selbst ein Beobachtungspunkt. Wer den Server kontrolliert oder mitliest, sieht die Verbindungen. Deshalb: eigener Zweck, eigener Server, keine anderen Dienste darauf.

## Domain und DNS (optional)

Mit eigener Domain:
```bash
sudo apt install -y certbot
sudo certbot certonly --standalone -d canary.example.net
```
Für DNS-Mitschnitt eine Subdomain delegieren:
```
; in der Zone example.net
canary-dns   IN NS   canary.example.net.
```
Auf dem Server `dnsmasq`:
```
# /etc/dnsmasq.d/canary.conf
port=53
log-queries
log-facility=/var/log/canary/dns.log
no-resolv
no-hosts
address=/#/
```
Das beantwortet jede Anfrage mit einer leeren Antwort und protokolliert die Anfrage. Kommt während des Tests eine Anfrage für `irgendwas.canary-dns.example.net` an, hat der Rechner lokal aufgelöst, statt über Tor.

## Testverfahren in der VM

Alles über `tests/vm/run-in-vm.ps1` oder von Hand in der VM.

| Test | Befehl | Erwartung |
|---|---|---|
| Baseline ohne Schutz | `curl -s https://canary.example.net/echo` | echte IP des Anschlusses erscheint |
| Über VPN + Tor | `curl -s --socks5-hostname 127.0.0.1:9250 https://canary.example.net/echo` | Exit-IP, nicht die echte |
| Direkt-Leak bei scharfer Firewall | `curl -s --max-time 5 https://canary.example.net/echo` | Timeout, kein Eintrag im Log |
| UDP/QUIC-Leak | `nc -u canary.example.net 9999` plus ein Zeichen | kein Eintrag im Log |
| IPv6-Leak | `curl -s -6 https://canary.example.net/echo` | Timeout, kein Eintrag |
| DNS-Leak | `nslookup test.canary-dns.example.net` | kein Eintrag im DNS-Log |

Nach jedem Test: Log-Zeilen mit Zeitstempel notieren (ohne IP in Produkt-Logs), Log leeren.

## Grenzen

- Der Canary beweist nur, was bei ihm ankommt. Ein Leak an einen anderen Server bleibt unsichtbar.
- NAT und CGNAT können Quell-Ports verändern; die IP bleibt aussagekräftig, der Port nicht.
- Ein kompromittierter Client kann den Test bestehen und trotzdem Schaden anrichten.
- Der Canary ist kein Anonymisierungsdienst und keine VPN-Gegenstelle. Er sammelt absichtlich Verbindungsdaten; deshalb kurzlebig und mager konfiguriert.
