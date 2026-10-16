<p align="center">
  <img src="frontend/public/logo.png" width="96" height="96" alt="iPredict logo" />
</p>

<h1 align="center">iPredict - Prediction Market on Stellar</h1>

<p align="center">
  <a href="https://github.com/AkanEf);/ipredict-stellar/actions"><img src="https://img.shields.io/github/actions/workflow/status/AkanEf);/ipredict-stellar/ci.yml?branch=main&label=CI&logo=github" alt="CI" /></a>
  <img src="https://img.shields.io/badge/Stellar-Soroban-blue?logo=stellar" alt="Stellar" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License" />
  <img src="https://img.shields.io/badge/tests-207%20passing-brightgreen" alt="Tests" />
</p>

**Predict. Win or Lose — You Always Earn.** Decentralized prediction market on Stellar with near-zero fees and 5-second finality.

---

## Live Demo

**Frontend:** [https://ipredict-stellar.vercel.app](https://ipredict-stellar.vercel.app)

## Demo Video

🎬 [Watch Full MVP Flow on Loom](https://www.loom.com/share/bbfa51fde81645eb9ca13eeba82bc695) — *Wallet connect → browse markets → place bet → view leaderboard → claim reward → referral flow*

## Deployed Contracts (Stellar Mainnet)

| Contract | Address | Explorer |
|----------|---------|----------|
| Prediction Market | `CDGNPRYTFDXJLWZE4YDKZXW4IEN2RLPSE4N7VM5HJ7NLPL2QC45GIXI5` | [stellar.expert](https://stellar.expert/explorer/public/contract/CDGNPRYTFDXJLWZE4YDKZXW4IEN2RLPSE4N7VM5HJ7NLPL2QC45GIXI5) |
| IPREDICT Token | `CAYL4TKNRMXAX5ZLQGFEZ6XOC2QHTCTN5QC2SB5BEEHLVO6SDU2UBLRH` | [stellar.expert](https://stellar.expert/explorer/public/contract/CAYL4TKNRMXAX5ZLQGFEZ6XOC2QHTCTN5QC2SB5BEEHLVO6SDU2UBLRH) |
| Referral Registry | `CAGJVX6EXMCKKWDJCQFIEJ34CZTHZOGLWJM6KQTGDEXEO723CJZ5773H` | [stellar.expert](https://stellar.expert/explorer/public/contract/CAGJVX6EXMCKKWDJCQFIEJ34CZTHZOGLWJM6KQTGDEXEO723CJZ5773H) |
| Leaderboard | `CCWWOQSDSO3XXLCMA6A2HYRUFYVNUJZ2HPAMFQSPOB4JWYIBY2HWVTOB` | [stellar.expert](https://stellar.expert/explorer/public/contract/CCWWOQSDSO3XXLCMA6A2HYRUFYVNUJZ2HPAMFQSPOB4JWYIBY2HWVTOB) |

> **Network:** Stellar Mainnet (Public) | **Admin:** `GDZ4VJWNJPLNU3PAWDYX3V5XNATO7X257DPHWRPFXSCCNEUZ7QTXIIUI` | **7 seed markets live** | All contracts upgradeable
>
> **Native XLM SAC:** `CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA` (mainnet)

---

## TLS Certificate Monitoring & Renewal

Production API and frontend endpoints use Let's Encrypt TLS certificates that must be renewed periodically. Expired certificates cause browser warnings and full platform downtime, so renewal and expiry monitoring are automated.

### Automated Renewal (certbot)

Certbot runs on a systemd timer and renews certificates before they expire. A deploy hook reloads the web server only when a certificate is actually renewed.

```ini
# /etc/systemd/system/certbot-renew.timer
[Unit]
Description=Run certbot twice daily

[Timer]
OnCalendar=*-*-* 00,12:00:00
RandomizedDelaySec=3600
Persistent=true

[Install]
WantedBy=timers.target
```

```ini
# /etc/systemd/system/certbot-renew.service
[Unit]
Description=Certbot renewal

[Service]
Type=oneshot
ExecStart=/usr/bin/certbot renew --quiet --deploy-hook /usr/local/bin/reload-tls.sh
```

```bash
#!/usr/bin/env bash
# /usr/local/bin/reload-tls.sh — reload web server after a successful renewal
set -euo pipefail
systemctl reload nginx
```

Enable the timer once on each host:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now certbot-renew.timer
```

### Expiry Monitoring & Alerts

A cron job checks every certificate daily and alerts operations when a certificate has **fewer than 15 days** remaining, or when the automated renewal itself fails. The check exits non-zero on any alert so the failure is also visible to the scheduler.

```bash
#!/usr/bin/env bash
# /usr/local/bin/check-tls-expiry.sh
# Alerts operations when a TLS certificate has < 15 days remaining.
set -euo pipefail

THRESHOLD_DAYS=15
ALERT_WEBHOOK="${ALERT_WEBHOOK:?ALERT_WEBHOOK must be set}"
HOSTS=("api.ipredict-stellar.app:443" "ipredict-stellar.vercel.app:443")

fail=0
for host in "${HOSTS[@]}"; do
  end_date=$(echo | openssl s_client -servername "${host%%:*}" -connect "$host" 2>/dev/null \
    | openssl x509 -noout -enddate | cut -d= -f2)
  if [ -z "$end_date" ]; then
    msg="TLS check failed: could not read certificate for $host"
    curl -fsS -X POST -H 'Content-Type: application/json' \
      -d "{\"text\":\"$msg\"}" "$ALERT_WEBHOOK" || true
    fail=1
    continue
  fi
  end_epoch=$(date -d "$end_date" +%s)
  now_epoch=$(date +%s)
  days_left=$(( (end_epoch - now_epoch) / 86400 ))
  if [ "$days_left" -lt "$THRESHOLD_DAYS" ]; then
    msg="TLS certificate for $host expires in ${days_left} days (< ${THRESHOLD_DAYS})"
    curl -fsS -X POST -H 'Content-Type: application/json' \
      -d "{\"text\":\"$msg\"}" "$ALERT_WEBHOOK" || true
    fail=1
  fi
done
exit "$fail"
```

```cron
# /etc/cron.d/tls-expiry — daily certificate expiry check
0 6 * * * root ALERT_WEBHOOK=https://hooks.example.com/ops /usr/local/bin/check-tls-expiry.sh
```

### Alerting on Renewal Failure

The renewal service is paired with an `OnFailure` unit so operations are paged if certbot cannot renew a certificate:

```ini
# /etc/systemd/system/certbot-renew.service.d/alert.conf
[Unit]
OnFailure=certbot-renew-alert.service
```

```ini
# /etc/systemd/system/certbot-renew-alert.service
[Unit]
Description=Alert operations that certbot renewal failed

[Service]
Type=oneshot
ExecStart=/usr/bin/curl -fsS -X POST -H 'Content-Type: application/json' \
  -d '{"text":"certbot renewal FAILED — TLS certificates may expire"}' \
  https://hooks.example.com/ops
```

---
## User Validation

> The table below records our **pre-launch testnet validation** — 30+ real users
> who tested the full flow (bet, claim, referral) on Stellar Testnet before the
> mainnet launch. These are historical testnet wallets, kept here as proof of
> product testing. Live mainnet activity is on the contracts listed above.

### Pre-Launch Testnet Users

| # | Wallet Address | Action | Points |
|---|----------------|--------|--------|
| 1 | `GDHQ6TNWZ4V2JVCDWEUVW7YKFBXCOQZRRUCT27LAKES3PGOE6JSZMSMD` | Admin — Created markets, placed bets, claimed rewards | 83 |
| 2 | `GA5OIFR4TLRJC54B3FIDWHIQKVFGVOWQFE3NXIIGCGLSGTJRUY6RHR2Q` | Placed bets, claimed rewards, referral registration | 38 |
| 3 | `GAIWJS35TTF2XMYGHY6AX6YBGI4IG3RWSEL2FEXV3DESW5RITGPWIIED` | Placed bets, claimed rewards, referral registration | 29 |
| 4 | `GA7GFCGUDLWZ6DIDCNERHA5CI7MATE6NT3MKDVJKFYIM5Y543OTLO3UY` | Placed bets, referral registration | 20 |
| 5 | `GDMA5FFWVJIOTBLO4XZWW5O4DWPOO33TLUOFCQQ7GPGTIRU6WXHCT4ZM` | Placed bets, referral registration | 17 |
| 6 | `GCSX2AGJRP747DWRCB3YYCKTXHU5CC6YNQUZPOCWWGA4DTK2PY5R6MUH` | Placed bets, referral registration | 8 |
| 7 | `GDKC2TFZVEJ7M7W2DLJZYFHK4AMHFX4JZKDP5JPQODBR7OT7C3RULXXA` | Placed bets, referral registration | 8 |
| 8 | `GAKY6JQSSCJZ3MQENCYAWIOZWGDKQJI7BCP6ZB35WTQZCMTYZ6M67UQG` | Placed bets, referral registration | 8 |
| 9 | `GCR32ST6YU3PLIOW7R6LRCB7ZMSSLAROMZGRQTK5CSOGW4GMIGMQGN5V` | Referral registration | 5 |
| 10 | `GA2LXPEKNP4IYZ2UKD4YUOGWPYMTNHGNXQRR2X3XT52XKQ5UV6BZKNFR` | Referral registration | 5 |
| 11 | `GCY4URAJ2JKE3UHCAEZHG5CTMVBS54S5IWQEWU3IIJX2ROOZ55YQYSOV` | Referral registration | 5 |
| 12 | `GDSSMRFIF56YMGR7DJY2DBQNQNLGJHJ5AW2DP3QSNVJVR4US6DTTM5YV` | Referral registration | 5 |
| 13 | `GB7M36KOEMNREMS4KMCXKUUYSHAT4JWEXLQJOFOEV3OA5SCUYYFEPGVO` | Referral registration | 5 |
| 14 | `GDD7MQAP2OE4NTEHLKNSUGIUMJN24RQL72ZTJAYGCDVULNP67JLUYGQW` | Referral registration | 5 |
| 15 | `GDUF4W7CEIM3JFGZFW4PTLWGCY3G3JD5P4WZMA4C6QFODLC7M6ASC3PU` | Referral registration | 5 |
| 16 | `GCXEQC2DIPFWPVR4JMROCYSB6YMP6TW3Q2YEV3IXTCRE4ZGSURKVL5LT` | Referral registration | 5 |
| 17 | `GBDFA2O72OOCUII5NWWKQ6G7V2EZ6D3GDWIYMCMQFVOAHFVAAM33ZOFZ` | Referral registration | 5 |
| 18 | `GACER5XK5SMHR64FZOELYPPLJVRWFBDY5UWCBOEBSLNK266Q2L45YD2P` | Referral registration | 5 |
| 19 | `GDKZYMCENO7VCOHIZRAV5OP26X5F6KV5ETT7L35WW6G7QWD4TVI72CXE` | Referral registration | 5 |
| 20 | `GCLS42CXE5GJGVNPBTF7YIJLI2QGO7LLZK5QFAGO7KPXRNQOLIUBZHFP` | Referral registration | 5 |
| 21 | `GAW5CACDFYE4T4U26IYYSEJTGXLU4HU5LREGTM4D5NLRPZSK5Y4YBVCE` | Referral registration | 5 |
| 22 | `GAWDCG7UCCJH4AOZPD45R4VPZG3XH7NVPIMZGNXT2BDKHMPRK6WSZXVV` | Referral registration | 5 |
| 23 | `GDQIMRXKQDDPXAAU6RQIXFG34GZPV2TEZWJTPE6IPSISG4P5GF5CZZ3B` | Referral registration | 5 |
| 24 | `GAEAQCVLZML7E74YYSA4VMACJWFZKABU774BDZ2PIOZC7QVQ3R3LKLPO` | Referral registration | 5 |
| 25 | `GAYU7KUVSIO2CQDMPN7GB62GTJTG73UYOBVO6RVV6WPOO45R2PTGABJI` | Referral registration | 5 |
| 26 | `GDJWQ5PBXYYRVQRMLZVRSKRCSJJBZNK2V7S5UN323JCUDK3D54YBUTM2` | Referral registration | 5 |
| 27 | `GCO7CTBLSFEGZKBYNWO34COSOON3FRAOM44HF23XLRV5QM72ZGV35ZL4` | Referral registration | 5 |
| 28 | `GBE663I55YXLC7U26GZBDUBHYXOYQFUXSXALDM2BXQRFUOPMFASGDGWQ` | Referral registration | 5 |
| 29 | `GBX3HFO3J6IPOXPH2BWPWKNSCFNMKGANMFJPBFC3JACIMBJM6UISGNZK` | Referral registration | 5 |
| 30 | `GASJLRNNMPYN3AIA6MFB6ETRDI5H5C36QRRXLFK62XZUJWGVK4YWNF6R` | Referral registration | 5 |
| 31 | `GBQI3WD2YKYUWB4MXOO3QP7DPENLV3WD32EHA43VV3US2DCBOWEPV7GW` | Referral registration | 5 |

## Screenshot of Leaderboard with 30+ Pre-Launch Testnet Users
![Leaderboard of 30+ Users](image_of_30+_testnet_user.png)

*Pre-launch testnet wallets verifiable on [Stellar Expert Testnet Explorer](https://stellar.expert/explorer/testnet). Live mainnet contracts are linked in the Deployed Contracts section above.*

### User Feedback
- **Form Link that I shared with users:** [https://forms.gle/WrTyoCzQ6LJegL8a9](https://forms.gle/WrTyoCzQ6LJegL8a9)
- **The Spreadsheet with feedback:** [https://docs.google.com/spreadsheets/d/1rGhq1rgeDPiF6hI4iDL2eFt0vUMBDA6LSm9zS2txpiA/edit?usp=sharing](https://docs.google.com/spreadsheets/d/1rGhq1rgeDPiF6hI4iDL2eFt0vUMBDA6LSm9zS2txpiA/edit?usp=sharing)

---

## User Feedback Implementation: Added more filtering and default sorting options to the leaderboard on Active
|*User Name*| *User Email* | *User Wallet Adddress* | *Commit ID*|
|----------------------|-------------------------|-------------------------|----------------|
| best_bettor | `tesinger96@gmail.com` | `GA7GFCGUDLWZ6DIDCNERHA5CI7MATE6NT3MKDVJKFYIM5Y543OTLO3UY` | `77c37784589046eeeb917a9015085dfa6fb26a52` |


## Advance Feature Added: Fee Sponsorship, Gasless Transactions
  > we added a Fee Sponsorship, Gasless Transactions feature to our MVP, allowing users to interact with the prediction market without needing to hold XLM for transaction fees. This significantly lowers the barrier to entry, enabling anyone to place bets and participate in the market seamlessly. The platform covers the transaction fees on behalf of the users, creating a more inclusive and user-friendly experience.


## Features

- **Binary Prediction Markets** — Bet YES or NO on any question with XLM
- **Inclusive Reward System** — Both winners AND losers earn points + IPREDICT tokens
- **Onchain Referral Program** — Share your link, earn 0.5% of every referred bet + bonus points
- **Real-Time Leaderboard** — Rankings by points, volume, and win rate from onchain data
- **Social Sharing** — One-tap sharing to X, Telegram, WhatsApp after every bet
- **4 Independent Smart Contracts** — Single responsibility, ind

/* … truncated 11519 chars — edit only what you need near the top … */
