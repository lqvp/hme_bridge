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

## Authentication Methods

This bridge supports two authentication methods:

1. **Direct Cookie Authentication (Legacy):** Suitable for clients like Bitwarden that only support a single API key field. You pass the iCloud cookie JSON directly in the `authentication` header.
2. **Centralized Token Authentication (Recommended):** A more secure and flexible method using a KV store to manage credentials. Clients authenticate using a short-lived token. This is ideal for multi-client or multi-account setups and is fully compatible with Bitwarden.

---

## Method 1: Direct Cookie Authentication (for Bitwarden)

This is the simplest method for Bitwarden users.

### How to Configure Bitwarden

1. Go to **Generator** -> **Username**.
2. Select **Forwarded email alias**.
3. For **Service**, select **SimpleLogin**.
4. For **Email domain**, enter your Cloudflare Worker's URL
   (e.g., `https://hme-bridge.your-username.workers.dev`).
   **Note:** Make sure to include `https://` at the beginning, and do **not** add a trailing `/`.
5. For **API Key**, paste your iCloud Cookie JSON here. See the next section for instructions on how to get it.

### How to get the API Key (iCloud Cookies)

To authenticate with iCloud, you must provide three essential cookies from your browser session.

1. Install a cookie editor extension that can export in **JSON format**. We recommend **Get cookies.txt LOCALLY**.
    - [Source Code](https://github.com/kairi003/Get-cookies.txt-LOCALLY)
    - [Chrome](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc)
    - [Firefox Link](https://addons.mozilla.org/en-US/firefox/addon/get-cookies-txt-locally/)
2. Log in to [icloud.com](https://www.icloud.com/).
3. Open the extension and export cookies for the current site as **JSON**.
4. Open the exported JSON file/text.
5. Find the cookie objects corresponding to the following three `name`s:
    - `X-APPLE-DS-WEB-SESSION-TOKEN`
    - `X-APPLE-WEBAUTH-TOKEN`
    - `X-APPLE-WEBAUTH-USER`
6. You can simply copy and paste the entire exported JSON directly into Bitwarden — the server-side parser will automatically extract the required tokens.
   However, if you prefer a stricter setup, create a new JSON array containing **only these three cookie objects**.
7. Paste the resulting JSON array (or the full JSON if using the simple method) into the API Key field in Bitwarden.

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

---

## Method 2: Centralized Token Authentication (Recommended)

This method is more secure and flexible, allowing you to manage multiple accounts or clients. It is **fully compatible with Bitwarden**.

### How it Works for Bitwarden Users

Instead of pasting your full iCloud cookie data into Bitwarden's API Key field, you first store the cookie data in the bridge's secure KV store and generate a simple token. Then, you just use that short token as your API Key in Bitwarden.

This has two main benefits:

- **Enhanced Security:** Your sensitive iCloud cookie is not stored in the Bitwarden client.
- **Simplified Management:** If your cookie expires, you only need to update it in one place (the KV store), and all your clients (including Bitwarden) will work again without any changes.

### 1. Configure KV Namespace and Admin Token

First, you need to set up a Cloudflare KV namespace (for storing credentials) and a secret token (for managing them).

#### A. Create KV Namespace

Run the following commands in your terminal. The first command creates the KV namespace for production, and the second creates one for local testing with `wrangler dev`.

```bash
# For production
wrangler kv:namespace create HME_BRIDGE_CREDS

# For local testing
wrangler kv:namespace create HME_BRIDGE_CREDS --preview
```

After running each command, Wrangler will output a configuration block. Copy and paste both blocks into your `wrangler.toml` file. It should look something like this:

```toml
# wrangler.toml
[[kv_namespaces]]
binding = "HME_BRIDGE_CREDS"
id = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"

[[kv_namespaces]]
binding = "HME_BRIDGE_CREDS"
preview_id = "yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy"
```

#### B. Set Admin Token

Next, set a secret token for the admin API. Choose a strong, random string.

```bash
# For production
wrangler secret put ADMIN_TOKEN

# For local testing (creates a .dev.vars file)
echo 'ADMIN_TOKEN="your_super_secret_admin_token"' > .dev.vars
```

### 2. Manage Credentials via Admin API

A full set of CRUD (Create, Read, Update, Delete) endpoints are provided to manage your credentials. All requests must include the `X-Admin-Token` header.

#### A. Create a New Credential

This endpoint will add a new credential to the store and return it with a randomly generated token.

- **Endpoint:** `POST /admin/credentials`
- **Body:** A JSON object with a `label` (a friendly name for the credential) and the `cookie` data (as a JSON array).

**Example `curl` command:**
(Assumes your cookie data is in a file named `cookie.json`)

```bash
jq -n --arg label "My Personal Account" --argjson cookie "$(cat cookie.json)" \
  '{"label": $label, "cookie": $cookie}' | \
  curl -X POST https://your-worker.workers.dev/admin/credentials \
    -H "X-Admin-Token: your_super_secret_admin_token" \
    -H "Content-Type: application/json" \
    -d @-
```

**Successful Response:**
You will get a JSON response containing the new credential, including the generated `token`. Use this token in your client (e.g., Bitwarden API Key).

```json
{
  "label": "My Personal Account",
  "token": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
  "cookie": "[...]"
}
```

#### B. List All Credentials

- **Endpoint:** `GET /admin/credentials`

**Example `curl` command:**

```bash
curl https://your-worker.workers.dev/admin/credentials \
  -H "X-Admin-Token: your_super_secret_admin_token"
```

**Successful Response:** An array of all stored credential objects.

#### C. Update a Credential

- **Endpoint:** `PUT /admin/credentials/:token`
- **Body:** A JSON object with the updated `label` and `cookie`.

**Example `curl` command:**
(Updates the credential with the token `a1b2c3d4...`, using cookie data from `new_cookie.json`)

```bash
jq -n --arg label "My Updated Account" --argjson cookie "$(cat new_cookie.json)" \
  '{"label": $label, "cookie": $cookie}' | \
  curl -X PUT https://your-worker.workers.dev/admin/credentials/a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4 \
    -H "X-Admin-Token: your_super_secret_admin_token" \
    -H "Content-Type: application/json" \
    -d @-
```

#### D. Delete a Credential

- **Endpoint:** `DELETE /admin/credentials/:token`

**Example `curl` command:**

```bash
curl -X DELETE https://your-worker.workers.dev/admin/credentials/a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4 \
  -H "X-Admin-Token: your_super_secret_admin_token"
```

**Successful Response:** A simple text message like `Credential deleted`.

### 3. Client Configuration

#### For Bitwarden Users

After creating a credential and getting a token from the `POST /admin/credentials` endpoint, simply go to Bitwarden's generator settings and paste the returned **token** into the **API Key** field. The bridge will automatically detect that you are using a token and retrieve the corresponding cookie from the KV store.

#### For Custom Clients

Custom clients can use the token in two ways:

1. **Via `authentication` header (like Bitwarden):**

    ```
    authentication: <your_token>
    ```

2. **Via `Authorization` header (Standard method):**

    ```
    Authorization: Bearer <your_token>
    ```

This provides flexibility for different types of clients.

## LICENSE

[MIT](LICENSE)
