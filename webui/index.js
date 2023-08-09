let queue_rescan_enabled = true;
const queue_rescan_timeout = 1000;
let debug_enabled = false;
let config_editor_visible = false;
let login_forms_visible = false;
let format_list_initialized = false;
let known_formats = new Map()

let expanded_jobs = new Set()

function get_api_token() {
    return window.localStorage.getItem("api-token");
}

function set_api_token(new_api_token) {
    window.localStorage.setItem("api-token", new_api_token)
    let token_input_box = document.getElementById("api-token-input-box")
    token_input_box.value = new_api_token
}

function get_session_token() {
    return window.localStorage.getItem("session-token");
}

function set_session_token(new_session_token) {
    window.localStorage.setItem("session-token", new_session_token)
    let token_input_box = document.getElementById("session-token-input-box")
    token_input_box.value = new_session_token
}

function make_auth_headers() {
    return {
        "Api-Token": get_api_token(),
        "Session-Token": JSON.stringify(get_session_token()),
    };
}

function show_url_submit_msg(msg) {
    let url_submit_msg = document.getElementById("url-submit-msg")
    url_submit_msg.classList.remove("msg-hidden");
    url_submit_msg.textContent = msg;
    setTimeout(() => {
        url_submit_msg.classList.add("msg-hidden");
    }, 5000)
}

function formatBytes(bytes, decimals = 2) {
    if (!+bytes) return '0 Bytes'

    const k = 1024
    const dm = decimals < 0 ? 0 : decimals
    const sizes = ['Bytes', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB', 'EiB', 'ZiB', 'YiB']

    const i = Math.floor(Math.log(bytes) / Math.log(k))

    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`
}

function submit_url(event){
    event.preventDefault()
    let url_input_box = document.getElementById("url-input-box")
    let url = url_input_box.value
    url_input_box.value = ""

    let format_select = document.getElementById("submit-format-select")
    let format = format_select.value

    fetch("api/jobs/new", {
        method: "POST",
        headers: make_auth_headers(),
        body: new URLSearchParams({
            url: url,
            format: format
        })
    })
    .then(r => {
        if (r.status !== 202) {
            throw new Error("status code " + r.status)
        } else {
            console.info("Download successfully started.");
            show_url_submit_msg("URL submitted");
        }
    })
    .catch(e => {
        console.info("Failed to download: " + e);
        show_url_submit_msg("Failed to submit URL");
    });
    return false;
}

function fmt_video_format(format_id) {
    if (known_formats.has(format_id)) {
        return known_formats.get(format_id).display
    } else {
        format_id
    }
}

function update_api_token(event){
    event.preventDefault()
    let token_input_box = document.getElementById("api-token-input-box")
    set_api_token( token_input_box.value )
    set_session_token( "" )
    fetch("api/sessions/new",{
        method: "POST",
        body: new URLSearchParams({
            api_token: get_api_token(),
        })
    })
        .then(r => {
            if (r.status !== 200) {
                throw new Error("status code " + r.status)
            } else {
                return r.json()
            }
        })
        .then(data => {
            set_session_token( data )
        })
    return false;
}

function update_session(event) {
    event.preventDefault()
    let api_token_input_box = document.getElementById("api-token-input-box")
    set_api_token( api_token_input_box.value )
    let session_token_input_box = document.getElementById("session-token-input-box")
    set_session_token( session_token_input_box.value )
    return false;
}

function parseBool(str) {
    if (str === "true" || str === "1" || str === 1) {
        return true;
    } else if (str === "false" || str === "0" || str === 0) {
        return false;
    } else {
        return null;
    }
}

function submit_config(event){
    let form_data = new FormData(event.target);

    let json = {}
    for(let pair of form_data.entries()) {
        if (pair[0] === "download-dir") {
            json.download_folder = pair[1];
        }
        if (pair[0] === "temp-dir") {
            json.temp_folder = pair[1];
        }
        if (pair[0] === "start-with-os") {
            json.start_with_os = parseBool(pair[1]);
        }
        if (pair[0] === "show-announcements") {
            json.show_announcements = parseBool(pair[1]);
        }
        if (pair[0] === "num-retries") {
            json.num_automatic_retries = parseInt(pair[1]);
        }
        if (pair[0] === "num-workers") {
            json.num_download_workers = parseInt(pair[1]);
        }
        // TODO: extra args
    }
    //json.download_folder = document.getElementById("config-input-download-dir").value;
    //json.temp_folder = document.getElementById("config-input-temp-dir").value;
    //json.start_with_os = document.getElementById("config-input-start-with-os").value === "true";
    //json.show_announcements = document.getElementById("config-input-show-announcements").value === "true";
    //json.num_automatic_retries = parseInt(document.getElementById("config-input-num-retries").value);
    //json.num_download_workers = parseInt(document.getElementById("config-input-num-workers").value);
    json.extra_ytdl_arguments = [] // TODO
    // document.getElementById("config-input-extra-args").value

    fetch("api/config", {
        method: "POST",
        headers: make_auth_headers(),
        body: new URLSearchParams({
            value: JSON.stringify(json),
        })
    })
        .then(r => {
            if (r.status !== 202) {
                throw new Error("status code " + r.status)
            } else {
                console.info("Config successfully updated.");
                show_url_submit_msg("Config updated");
                set_config_editor_visible(false);
            }
        })
        .catch(e => {
            console.info("Failed to update config: " + e);
            show_url_submit_msg("Failed to update config");
        });
    return false;
}

function get_task_id_from_event(event) {
    const target = event.target;
    const data_container = target.closest(".data-container")
    return data_container.getAttribute("data-task-id");
}

function get_job_id_from_event(event) {
    const target = event.target;
    const data_container = target.closest(".data-container")
    return data_container.getAttribute("data-job-id");
}

function get_job_card_from_event(event) {
    const target = event.target;
    return target.closest(".task_job_card");
}

function cancel_task(event) {
    const task_id = get_task_id_from_event(event)
    fetch("api/tasks/cancel/" + task_id, {method: "POST", headers: make_auth_headers() })
    .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function pause_task(event) {
    const task_id = get_task_id_from_event(event)
    fetch("api/tasks/pause/" + task_id, { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function resume_task(event) {
    const task_id = get_task_id_from_event(event)
    fetch("api/tasks/resume/" + task_id, { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function retry_task(event) {
    const task_id = get_task_id_from_event(event)
    fetch("api/tasks/retry/" + task_id, { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function delete_task(event) {
    const task_id = get_task_id_from_event(event)
    fetch("api/tasks/delete/" + task_id, { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function cancel_job(event) {
    const job_id = get_job_id_from_event(event)
    fetch("api/jobs/cancel/" + job_id, { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function pause_job(event) {
    const job_id = get_job_id_from_event(event)
    fetch("api/jobs/pause/" + job_id, { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function resume_job(event) {
    const job_id = get_job_id_from_event(event)
    fetch("api/jobs/resume/" + job_id, { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function retry_job(event) {
    const job_id = get_job_id_from_event(event)
    fetch("api/jobs/retry/" + job_id, { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function delete_job(event) {
    const job_id = get_job_id_from_event(event)
    fetch("api/jobs/delete/" + job_id, { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function cancel_all_jobs() {
    fetch("api/jobs/cancel_all", { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function pause_all_jobs() {
    fetch("api/jobs/pause_all", { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function resume_all_jobs() {
    fetch("api/jobs/resume_all", { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function retry_all_jobs() {
    fetch("api/jobs/retry_all", { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function delete_all_jobs() {
    fetch("api/jobs/delete_all", { method: "POST", headers: make_auth_headers() })
        .then(() => {}).catch((e)=>{
        console.error(e);
    })
}

function request_server_shutdown() {
    fetch("api/shutdown_server", { method: "POST", headers: make_auth_headers() })
        .then(() => {
            set_queue_updates(false);

            let badge = document.getElementById("shutdown-server-badge");
            badge.disabled = true;
            badge.classList.remove("clickable-badge")
            badge.classList.add("disabled")

            let label = document.getElementById("shutdown-server-badge-label");
            label.textContent = "Shutdown requested";
        }).catch((e)=>{
        console.error(e);
    })
}

function collapse_job(event) {
    const job_id = get_job_id_from_event(event)
    expanded_jobs.delete(job_id)
    const card = get_job_card_from_event(event)
    const left_section = card.getElementsByClassName("left-section").item(0);
    left_section.classList.remove("left-section-job-expanded")
    left_section.classList.add("left-section-job-collapsed")
    const task_container = card.nextSibling
    task_container.classList.add("hidden")
}

function expand_job(event) {
    const job_id = get_job_id_from_event(event)
    expanded_jobs.add(job_id)
    const card = get_job_card_from_event(event)
    const left_section = card.getElementsByClassName("left-section").item(0);
    left_section.classList.remove("left-section-job-collapsed")
    left_section.classList.add("left-section-job-expanded")
    const task_container = card.nextSibling
    task_container.classList.remove("hidden")
}

function get_task_logs(task_id, log_kind) {
    let url = "api/tasks/get_" + log_kind + "/" + task_id;
    // We need our auth headers, so simple window.open won't work.
    // So, the hack:
    // 1. Download file to cache, using our auth headers in request
    // 2. Open file from cache
    fetch(url, { headers: make_auth_headers() })
        .then(resp => {
            return resp.blob()
        })
        .then(blob => {
            const _url = window.URL.createObjectURL(blob);
            window.open(_url, "_blank").focus();
        })
}

const ServerStatusKind = {
    Offline: "offline",
    Unauthorized: "unauthorized",
    Error: "error",
    Ok: "ok",
};

function set_server_status(status_kind) {
    let text
    let color
    if (status_kind === ServerStatusKind.Error) {
        text = "Internal error";
        color = "#de2706"
    } else if (status_kind === ServerStatusKind.Offline) {
        text = "Offline";
        color = "#724c4c"
    } else if (status_kind === ServerStatusKind.Unauthorized) {
        text = "Unauthorized";
        color = "#c05705"
    } else if (status_kind === ServerStatusKind.Ok) {
        text = "Ok";
        color = "rgb(100 116 139)"
    } else {
        text = "???";
        color = "#f0f"
    }

    const label = document.getElementById("server-status-text");
    label.textContent = text;
    label.style.backgroundColor = color;
}

function make_job_card(template_card, job) {
    let card = template_card.cloneNode(true);
    card.removeAttribute("id");
    card.setAttribute("data-job-id", job.job_id);
    card.classList.remove("hidden");

    const left_section = card.getElementsByClassName("left-section").item(0);
    let card_collapse_class
    if (expanded_jobs.has(job.job_id)) {
        card_collapse_class = "left-section-job-expanded"
    } else {
        card_collapse_class = "left-section-job-collapsed"
    }
    left_section.classList.add(card_collapse_class)

    const collapse_button = card.getElementsByClassName("control-collapse").item(0);
    collapse_button.onclick = collapse_job;

    const expand_button = card.getElementsByClassName("control-expand").item(0);
    expand_button.onclick = expand_job;

    const id_field = card.getElementsByClassName("yt-info-task-job-id").item(0);
    id_field.textContent = job.job_id;
    if (debug_enabled) {
        const debug_info = card.getElementsByClassName("yt-info-debug").item(0);
        debug_info.classList.remove("hidden");
    }

    const info_format = card.getElementsByClassName("yt-info-format").item(0);
    info_format.textContent = "[" + fmt_video_format(job.format) + "]"

    const info_title = card.getElementsByClassName("yt-info-title").item(0);
    info_title.textContent = job.title

    const info_url = card.getElementsByClassName("yt-info-link").item(0);
    info_url.setAttribute("href", job.url);
    info_url.textContent = job.url;

    if (job.thumbnail !== "") {
        const thumbnail = card.getElementsByClassName("card-thumbnail").item(0);
        thumbnail.src = job.thumbnail;
    }

    let status;
    let status_text = job.status;
    if (job.status === "Done") {
        status = "status-done";
    } else if (job.status === "PartiallyDone") {
        status = "status-partially-done";
        status_text = "Partially Done";
    } else if (job.status === "Failed") {
        status = "status-failed";
    } else if (job.status === "Cancelled") {
        status = "status-cancelled";
    } else if (job.status === "Processing") {
        status = "status-processing";
        status_text = "Processing"
    } else { // "Waiting"
        status = "status-waiting";
    }

    const status_badge_text = card.getElementsByClassName("card-status-text").item(0);
    status_badge_text.textContent = status_text

    const status_badge = card.getElementsByClassName("card-status-badge").item(0);
    status_badge.classList.add(status);

    const created_at = new Date(Date.parse(job.created_at));
    let finished_at;
    if (job.finished_at != null) {
        finished_at = new Date(Date.parse(job.finished_at));
    } else {
        finished_at = new Date(Date.now());
    }
    const elapsed = Math.floor((finished_at.getTime() - created_at.getTime()) / 1000);
    const elapsed_label = card.getElementsByClassName("text-elapsed-time").item(0);
    elapsed_label.textContent = elapsed + " seconds";

    const num_done = job.tasks.filter(task => {
        return task.kind !== "FetchUrlContents" && task.status === "Done";
    }).length
    const num_total = job.tasks.filter(task => {
        return task.kind !== "FetchUrlContents";
    }).length

    const progress_label = card.getElementsByClassName("text-progress").item(0);
    progress_label.textContent = `${num_done} out of ${num_total}`;

    let control_panel = card.getElementsByClassName("control-panel").item(0);

    let control_cancel = control_panel.getElementsByClassName("control-cancel").item(0);
    if (job.status === "Waiting" || job.status === "Processing") {
        control_cancel.onclick = cancel_job;
    } else {
        control_cancel.classList.add("control-btn-disabled");
        control_cancel.disabled = true;
    }

    let control_pause = control_panel.getElementsByClassName("control-pause").item(0);
    if (job.status === "Waiting" || job.status === "Processing") {
        control_pause.onclick = pause_job;
    } else {
        control_pause.classList.add("control-btn-disabled");
        control_pause.disabled = true;
    }

    let control_resume = control_panel.getElementsByClassName("control-resume").item(0);
    if (job.status === "Paused") {
        control_resume.onclick = resume_job;
    } else {
        control_resume.classList.add("control-btn-disabled");
        control_resume.disabled = true;
    }

    let control_retry = control_panel.getElementsByClassName("control-retry").item(0);
    if (job.status === "PartiallyDone" || job.status === "Cancelled" || job.status === "Paused" || job.status === "Failed") {
        control_retry.onclick = retry_job;
    } else {
        control_retry.classList.add("control-btn-disabled");
        control_retry.disabled = true;
    }

    let control_delete = control_panel.getElementsByClassName("control-delete").item(0);
    control_delete.onclick = delete_job;

    return card;
}

function make_task_card(template_card, task) {
    let card = template_card.cloneNode(true);
    card.removeAttribute("id");
    card.setAttribute("data-task-id", task.task_id);
    card.classList.remove("hidden");

    if (task.kind === "FetchUrlContents") {
        card.classList.add("debug-element")
        if (!debug_enabled) {
            card.classList.add("hidden");
        }
    }

    const left_section = card.getElementsByClassName("left-section").item(0);
    left_section.classList.add("left-section-task")

    const label_task_number = card.getElementsByClassName("label-task-number").item(0);
    label_task_number.textContent = task.index

    const id_field = card.getElementsByClassName("yt-info-task-job-id").item(0);
    id_field.textContent = task.task_id;
    if (debug_enabled) {
        const debug_info = card.getElementsByClassName("yt-info-debug").item(0);
        debug_info.classList.remove("hidden");
    }

    const info_format = card.getElementsByClassName("yt-info-format").item(0);
    info_format.textContent = "[" + fmt_video_format(task.format) + "]"

    const info_title = card.getElementsByClassName("yt-info-title").item(0);
    info_title.textContent = task.title

    const info_url = card.getElementsByClassName("yt-info-link").item(0);
    info_url.setAttribute("href", task.url);
    info_url.textContent = task.url;

    if (task.thumbnail !== "") {
        const thumbnail = card.getElementsByClassName("card-thumbnail").item(0);
        thumbnail.src = task.thumbnail;
    }

    let status;
    let status_text = task.status;
    if (task.status === "Done") {
        status = "status-done";
    } else if (task.status === "Failed") {
        status = "status-failed";
    } else if (task.status === "Cancelled") {
        status = "status-cancelled";
    } else if (task.status === "Processing") {
        status = "status-processing";
        if (task.progress != null) {
            if (task.progress.percent === 100) {
                status_text = "Converting";
            } else {
                status_text = `Downloading ${task.progress.percent}%`
            }
        } else {
            status_text = "Downloading"
        }
    } else { // "Waiting"
        status = "status-waiting";
    }

    const status_badge_text = card.getElementsByClassName("card-status-text").item(0);
    status_badge_text.textContent = status_text

    const status_badge = card.getElementsByClassName("card-status-badge").item(0);
    status_badge.classList.add(status);

    const created_at = new Date(Date.parse(task.created_at));
    let finished_at;
    if (task.finished_at != null) {
        finished_at = new Date(Date.parse(task.finished_at));
    } else {
        finished_at = new Date(Date.now());
    }
    const elapsed = Math.floor((finished_at.getTime() - created_at.getTime()) / 1000);
    const elapsed_label = card.getElementsByClassName("text-elapsed-time").item(0);
    elapsed_label.textContent = elapsed + " seconds";

    if (task.progress != null) {
        const estimate = formatBytes(task.progress.bytes_estimate);
        const downloaded = formatBytes(task.progress.bytes_downloaded);
        const progress_label = card.getElementsByClassName("text-progress").item(0);
        progress_label.textContent = `${downloaded} / ${estimate}`;
    }

    let control_panel = card.getElementsByClassName("control-panel").item(0);

    let control_cancel = control_panel.getElementsByClassName("control-cancel").item(0);
    if (task.status === "Waiting" || task.status === "Processing") {
        control_cancel.onclick = cancel_task;
    } else {
        control_cancel.classList.add("control-btn-disabled");
        control_cancel.disabled = true;
    }

    let control_pause = control_panel.getElementsByClassName("control-pause").item(0);
    if (task.status === "Waiting" || task.status === "Processing") {
        control_pause.onclick = pause_task;
    } else {
        control_pause.classList.add("control-btn-disabled");
        control_pause.disabled = true;
    }

    let control_resume = control_panel.getElementsByClassName("control-resume").item(0);
    if (task.status === "Paused") {
        control_resume.onclick = resume_task;
    } else {
        control_resume.classList.add("control-btn-disabled");
        control_resume.disabled = true;
    }

    let control_retry = control_panel.getElementsByClassName("control-retry").item(0);
    if (task.status === "Cancelled" || task.status === "Paused" || task.status === "Failed") {
        control_retry.onclick = retry_task;
    } else {
        control_retry.classList.add("control-btn-disabled");
        control_retry.disabled = true;
    }

    let control_delete = control_panel.getElementsByClassName("control-delete").item(0);
    control_delete.onclick = delete_task;

    let control_stdout = control_panel.getElementsByClassName("control-stdout").item(0);
    let control_stderr = control_panel.getElementsByClassName("control-stderr").item(0);
    control_stdout.classList.remove("hidden")
    control_stderr.classList.remove("hidden")

    if (task.status === "Waiting" || task.status === "Done" || task.status === "Processing") {
        control_stdout.classList.add("control-btn-disabled");
        control_stdout.disabled = true;
        control_stderr.classList.add("control-btn-disabled");
        control_stderr.disabled = true;
    } else {
        control_stdout.onclick = () => {
            get_task_logs(task.task_id, "stdout")
        }
        control_stderr.onclick = () => {
            get_task_logs(task.task_id, "stderr")
        }
    }

    return card;
}

function display_jobs(jobs) {
    const display_container = document.getElementById("new-display-container");
    display_container.innerHTML = "";
    const template_card = document.getElementById("template-card");
    const card_container_template = document.getElementById("card-container");
    const card_task_container_template = document.getElementById("card-task-container");

    jobs.forEach(job => {
        const card_container = card_container_template.cloneNode(true)
        card_container.removeAttribute("id")
        card_container.classList.remove("hidden")
        const card_task_container = card_task_container_template.cloneNode(true)
        card_task_container.removeAttribute("id")

        card_container.appendChild(make_job_card(template_card, job));
        card_container.appendChild(card_task_container)

        let task_index = 1;
        job.tasks.forEach(task => {
            if (task.kind === "FetchUrlContents") {
                task.index = 0;
            } else {
                task.index = task_index;
                task_index += 1;
            }
        });
        job.tasks.forEach(task => {
            task.progress = job.progress[task.task_id]
            card_task_container.appendChild(make_task_card(template_card, task));
        });
        if (expanded_jobs.has(job.job_id)) {
            card_task_container.classList.remove("hidden")
        }

        display_container.appendChild(card_container);
    })
}

function register_formats(formats) {
    console.log(formats)

    formats.forEach(format => {
        known_formats.set(format.id, format);
    })

    const formats_container = document.getElementById("submit-format-select");
    const format_template = document.getElementById("format-select-template");
    formats.forEach(format => {
        const format_entry = format_template.cloneNode(true);
        format_entry.removeAttribute("id")
        format_entry.value = format.id;
        format_entry.textContent = format.display;
        formats_container.appendChild(format_entry)
    })
    formats_container.removeChild(format_template);
}

function try_initialize_format_list() {
    fetch("api/formats", {headers: make_auth_headers() })
        .then( r => {
            if (r.status === 200) {
                return r.json();
            } else {
                return undefined;
            }
        })
        .then(data => {
            if (data === undefined) {
                return;
            }
            register_formats(data);
            format_list_initialized = true;
        })
}

function queue_rescan() {
    if (!format_list_initialized) {
        try_initialize_format_list();
    }
    if (!queue_rescan_enabled) {
        setTimeout(queue_rescan, queue_rescan_timeout);
        return;
    }
    fetch("api/ping", {method:"POST"})
        .then(r => {
            if (r.status === 200) {
                return fetch("api/status", {headers: make_auth_headers() });
            } else {
                throw new Error("Server offline");
            }
        })
        .catch((e) => {
            set_server_status(ServerStatusKind.Offline);
        })
        .then(r => {
            if (r === undefined) {
                return undefined;
            }
            if (r.status === 200) {
                return r.json();
            } else {
                throw new Error("Unauthorized");
            }
        })
        .catch((e) => {
            set_server_status(ServerStatusKind.Unauthorized);
        })
        .then(json => {
            if (json === undefined) {
                return undefined;
            }
            document.getElementById("num-total").textContent = json.num_total;
            document.getElementById("num-active").textContent = json.num_active;
            document.getElementById("num-waiting").textContent = json.num_waiting;
            document.getElementById("num-done").textContent = json.num_done;
            document.getElementById("num-cancelled").textContent = json.num_cancelled;
            document.getElementById("num-failed").textContent = json.num_failed;

            return fetch("api/jobs/get_all", {headers: make_auth_headers() });
        })
        .then(r => {
            if (r === undefined) {
                return undefined;
            }
            if (r.status === 200) {
                return r.json()
            } else {
                throw new Error("Internal server error");
            }
        })
        .catch((e) => {
            set_server_status(ServerStatusKind.Error);
        })
        .then(jobs => {
            if (jobs === undefined) {
                return undefined;
            }
            display_jobs(jobs)
            set_server_status(ServerStatusKind.Ok);
        })
        .finally(()=>{
            setTimeout(queue_rescan, queue_rescan_timeout);
        })
}

function set_queue_updates(val) {
    queue_rescan_enabled = val;
    let label = document.getElementById("auto-update-status");
    if (queue_rescan_enabled) {
        label.textContent = "ON";
    } else {
        label.textContent = "OFF";
    }
}

function toggle_queue_updates() {
    if (queue_rescan_enabled) {
        set_queue_updates(false)
    } else {
        set_queue_updates(true)
    }
}

function set_debug_enabled(val) {
    debug_enabled = val;

    let label = document.getElementById("debug-status");
    if (debug_enabled) {
        label.textContent = "OFF";
    } else {
        label.textContent = "ON";
    }

    Array.prototype.forEach.call(document.getElementsByClassName("debug-element"), function(element) {
        if (debug_enabled) {
            element.classList.remove("hidden");
        } else {
            element.classList.add("hidden");
        }
    });
}

function toggle_debug() {
    if (debug_enabled) {
        set_debug_enabled(false)
    } else {
        set_debug_enabled(true)
    }
}

function set_config_editor_visible(val) {
    if (config_editor_visible) {
        config_editor_visible = false;
        document.getElementById("config-submit-form").classList.add("hidden");
    } else {
        fetch("api/config", {headers: make_auth_headers() })
            .then(r => r.json())
            .then(json => {
                config_editor_visible = true;
                document.getElementById("config-submit-form").classList.remove("hidden");

                document.getElementById("config-input-download-dir").value = json.download_folder;
                document.getElementById("config-input-temp-dir").value = json.temp_folder;
                document.getElementById("config-input-start-with-os").value = json.start_with_os;
                document.getElementById("config-input-show-announcements").value = json.show_announcements;
                document.getElementById("config-input-num-retries").value = json.num_automatic_retries;
                document.getElementById("config-input-num-workers").value = json.num_download_workers;
                document.getElementById("config-input-extra-args").value = "-- TODO --";

            })
    }
}

function toggle_config_editor() {
    if (config_editor_visible) {
        set_config_editor_visible(false)
    } else {
        set_config_editor_visible(true)
    }
}

function set_login_forms_visible(val) {
    if (login_forms_visible) {
        login_forms_visible = false;
        document.getElementById("login-forms-container").classList.add("hidden");
    } else {
        login_forms_visible = true;
        set_api_token(get_api_token());
        set_session_token(get_session_token());
        document.getElementById("login-forms-container").classList.remove("hidden");
    }
}

function toggle_login_forms() {
    if (login_forms_visible) {
        set_login_forms_visible(false)
    } else {
        set_login_forms_visible(true)
    }
}

function init(){
    set_debug_enabled(false)
    document.getElementById("url-submit-form").onsubmit = submit_url;
    document.getElementById("api-token-submit-form").onsubmit = update_api_token;
    document.getElementById("session-token-form").onsubmit = update_session;
    document.getElementById("config-submit-form").onsubmit = submit_config;
    document.getElementById("toggle-updates-badge").onclick = toggle_queue_updates;
    document.getElementById("toggle-debug-badge").onclick = toggle_debug;
    document.getElementById("btn-cancel-all").onclick = cancel_all_jobs;
    document.getElementById("btn-pause-all").onclick = pause_all_jobs;
    document.getElementById("btn-resume-all").onclick = resume_all_jobs;
    document.getElementById("btn-retry-all").onclick = retry_all_jobs;
    document.getElementById("btn-delete-all").onclick = delete_all_jobs;
    document.getElementById("shutdown-server-badge").onclick = request_server_shutdown;
    document.getElementById("edit-config-badge").onclick = toggle_config_editor;
    document.getElementById("open-login-badge").onclick = toggle_login_forms;
    queue_rescan()
    window.dispatchEvent(new Event("webui_init_done"))
}
window.onload = init;
console.log("Initialized!")
