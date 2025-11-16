# 🏥 Healthy

A simple device health monitor I use for my network.

## 🧰 Usage

```bash
cargo run -- --config devices.json --port 8000
```

Visit `http://localhost:8000` to view the dashboard.

## ⚙️ Configuration

Define your devices in `devices.json`:

```json
{
  domains: [
    "http://foo.bar.home"
  ],
  "devices": [
    {
      "ip": "192.168.1.1",
      "name": "router"
    },
    {
      "ip": "192.168.1.10",
      "name": "server"
    },
    {
      "ip": "2001:db8::1",
      "name": "ipv6-device"
    }
  ]
}
```

Each device is pinged every 5 seconds.

## 📡 API

- `GET /` - Web dashboard
- `GET /status` - JSON endpoint with all device and domain statuses
