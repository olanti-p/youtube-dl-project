const download_button_id = "ytd-dload-btn-id";
const download_button_image_url = browser.runtime.getURL("icons/download-img.svg");

async function get_tokens() {
    return await browser.storage.local.get([ "api_token", "session_token" ]);
}

function make_auth_headers(tokens) {
    return {
        "Api-Token": tokens.api_token,
        "Session-Token": JSON.stringify(tokens.session_token),
        "Access-Control-Allow-Origin": "*",
    };
}

function show_error(msg) {
    console.error("YTD_EXT: " + msg);
}

function make_download_button() {
    let download_button = document.createElement("button");
    download_button.id = download_button_id;
    download_button.classList.add("ytd-ext-button");
    download_button.classList.add("ytp-button");
    download_button.title = "Download video (d)";
    download_button.setAttribute("aria-keyshortcuts", "d");
    download_button.setAttribute("data-title-no-tooltip", "Download Video");
    download_button.setAttribute("aria-label", "Download Video (d)");

    fetch(download_button_image_url)
        .then(res => res.blob())
        .then(blob => {
            blob.text().then(data => {
                download_button.innerHTML = data;
            });
        });

    return download_button;
}

function get_video_id() {
    const page_manager = document.getElementById("page-manager");
    if (page_manager == null) {
        show_error("page_manager not found");
        return null;
    } else {
        const flexy = page_manager.getElementsByTagName("ytd-watch-flexy").item(0);
        return flexy.getAttribute("video-id");
    }
}

function get_canonical_url() {
    const video_id = get_video_id();
    if (video_id != null) {
        return "https://www.youtube.com/watch?v=" + video_id;
    } else {
        return null;
    }
}

function set_download_button_color(color)
{
    let path = document.getElementById("download-img-path");
    path.setAttribute("fill", color);
}

function set_download_button_job(job_id)
{
    let button = document.getElementById(download_button_id);
    button.setAttribute("data-id", job_id);
}

function get_download_button_job()
{
    let button = document.getElementById(download_button_id);
    return button.getAttribute("data-id");
}

function keep_checking_download_status(job_id) {
    if (job_id !== get_download_button_job()) {
        return;
    }
    get_tokens().then(tokens => {
        return fetch("http://localhost:8123/api/jobs/get/" + job_id, {
            headers: make_auth_headers(tokens),
        })
    })
    .then(r => {
        if (r.status !== 200) {
            throw new Error("status code " + r.status)
        }
        return r.json();
    }).then(json => {
        console.log(json)
        if (job_id !== get_download_button_job()) {
            return;
        }
        let keep_checking = false;
        if (json.status === "Done") {
            set_download_button_color("#0f0");
        } else if (json.status === "Waiting") {
            set_download_button_color("#410794");
            keep_checking = true;
        } else if (json.status === "Processing") {
            set_download_button_color("#fff200");
            keep_checking = true;
        } else {
            throw new Error("server reported failure");
        }
        if (keep_checking) {
            setTimeout(() => keep_checking_download_status(job_id), 1000);
        }
    })
    .catch(e => {
        console.warn("Failed to download: " + e);
        set_download_button_color("#f00");
    })
}

function start_download(video_url)
{
    console.log("Downloading " + video_url);
    set_download_button_color("#d5d5d5");
    get_tokens().then(tokens => {
        return fetch("http://localhost:8123/api/jobs/new", {
            method: "POST",
            headers: make_auth_headers(tokens),
            body: new URLSearchParams({
                url: video_url,
                format: "mp3",
            })
        })
    })
        .then(r => {
            if (r.status !== 202) {
                throw new Error("status code " + r.status)
            } else {
                console.info("Download successfully started.");
                return r.json()
            }
        })
        .then((data)=>{
            console.log(data)
            const job_id = data.job_id;
            set_download_button_job(job_id);
            keep_checking_download_status(job_id);
        })
        .catch(e => {
            console.info("Failed to download: " + e);
            set_download_button_color("#f00");
        });
}

function on_click()
{
    start_download(get_canonical_url());
}

function initialize()
{
    const right_controls = document.body.getElementsByClassName("ytp-right-controls").item(0);
    if (right_controls == null) {
        show_error("No right controls found!");
    } else {
        const subtitles_button = right_controls.getElementsByClassName("ytp-subtitles-button").item(0);
        if (subtitles_button == null) {
            show_error("No subtitles button found!");
        } else {
            let download_button = make_download_button();
            right_controls.insertBefore(download_button, subtitles_button);
            download_button.addEventListener("click", on_click);
        }
    }
}

initialize()
