
function get_api_token() {
    return window.localStorage.getItem("api-token");
}

function get_session_token() {
    return window.localStorage.getItem("session-token");
}

async function store_tokens_in_extension() {
    const api_token = get_api_token()
    const session_token = get_session_token()
    await browser.storage.local.set({
        api_token: api_token,
        session_token: session_token,
    })
    console.log("Extension tokens updated!")
}

async function initialize()
{
    {
        const api_token_form = document.getElementById("api-token-submit-form");
        const func = api_token_form.onsubmit;
        api_token_form.onsubmit = (event) => {
            const ret = func(event);
            store_tokens_in_extension();
            return ret;
        };
    }
    {
        const session_token_form = document.getElementById("session-token-form");
        const func = session_token_form.onsubmit;
        session_token_form.onsubmit = (event) => {
            const ret = func(event);
            store_tokens_in_extension();
            return ret;
        };
    }
    await store_tokens_in_extension()
}

window.addEventListener("webui_init_done", (event) => {
    initialize().then(()=>{})
});
