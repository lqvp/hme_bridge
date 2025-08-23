# HME Bridge

Bitwarden (or Vaultwarden) client's SimpleLogin integration to use iCloud Hide My Email.

## How to Use

1. Build and run this application.

    ```bash
    cargo run --release
    ```

    The server will start on `127.0.0.1:3000`.

2. Configure Bitwarden client.
    - Go to Generator -> Username.
    - Select "Forwarded email alias".
    - For "Service", select "SimpleLogin".
    - For "Email domain", enter your self-hosted server URL: `http://127.0.0.1:3000`
    - For "API Key", you need to provide your iCloud session cookies. See the next section.

## How to get the API Key (iCloud Cookies)

To authenticate with iCloud, you must provide three essential cookies from your browser session.

1. Install a cookie editor extension that can export in **JSON format**. We recommend **Get cookies.txt LOCALLY**.
    - [Chrome/Edge Link](https://github.com/kairi003/Get-cookies.txt-LOCALLY)
    - [Firefox Link](https://addons.mozilla.org/en-US/firefox/addon/get-cookies-txt-locally/)
    - [Repository](https://github.com/kairi003/Get-cookies.txt-LOCALLY)
2. Log in to [icloud.com](https://www.icloud.com/).
3. Open the extension and export cookies for the current site as **JSON**.
4. Open the exported JSON file/text.
5. Find the cookie objects corresponding to the following three `name`s:
    - `X-APPLE-DS-WEB-SESSION-TOKEN`
    - `X-APPLE-WEBAUTH-TOKEN`
    - `X-APPLE-WEBAUTH-USER`
6. Create a new JSON array containing **only these three cookie objects**.
7. Paste the entire JSON array string into the API Key field in Bitwarden.

**Example API Key format:**

```json
[
  {
    "domain": ".icloud.com",
    "name": "X-APPLE-DS-WEB-SESSION-TOKEN",
    "value": "AQFq...",
    "...": "..."
  },
  {
    "domain": ".icloud.com",
    "name": "X-APPLE-WEBAUTH-TOKEN",
    "value": "v=2:t=...",
    "...": "..."
  },
  {
    "domain": ".icloud.com",
    "name": "X-APPLE-WEBAUTH-USER",
    "value": "v=1:s=1...",
    "...": "..."
  }
]
