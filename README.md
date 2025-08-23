# HME Bridge for Cloudflare Workers

This project acts as a bridge between Bitwarden's SimpleLogin integration and Apple's iCloud Hide My Email (HME) service, running on the Cloudflare Workers serverless platform.

## How to Deploy

1. **Install Wrangler CLI:**
    If you don't have it already, install the Cloudflare Wrangler CLI.

    ```bash
    npm install -g wrangler
    ```

2. **Login to Cloudflare:**

    ```bash
    wrangler login
    ```

3. **Build and Deploy:**
    Deploy the worker to your Cloudflare account.

    ```bash
    wrangler deploy
    ```

    After deployment, Wrangler will output your worker's URL.

## How to Configure Bitwarden

1. Go to **Generator** -> **Username**.
2. Select **Forwarded email alias**.
3. For **Service**, select **SimpleLogin**.
4. For **Email domain**, enter your Cloudflare Worker's URL (e.g., `https://hme-bridge.your-username.workers.dev`).
5. For **API Key**, you need to provide your iCloud session cookies. See the next section.

## How to get the API Key (iCloud Cookies)

To authenticate with iCloud, you must provide three essential cookies from your browser session.

1. Install a cookie editor extension that can export in **JSON format**. We recommend **Get cookies.txt LOCALLY**.
    - [Chrome/Edge Link](https://github.com/kairi003/Get-cookies.txt-LOCALLY)
    - [Firefox Link](https://addons.mozilla.org/en-US/firefox/addon/get-cookies-txt-locally/)
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
```
