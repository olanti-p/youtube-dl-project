# Technical Specs

## Project structure
The project should consist of 3 major parts:
1. Background worker server which is always active on user's machine
2. Browser extension that lets the user download stuff in single click
3. Web UI that allows user to manage ongoing downloads and settings


## Common info
The project should allow users to download YouTube videos via single click.
There should be functionality to download as multiple possible formats:
1. MP3
2. Best audio (automatic)
3. MP4 HD
4. MP4 FullHD
5. Best video (automatic)
The download should happen in background, even after the browser or tab has been closed.
Once download finished, it's reported via system notification.


### Jobs
Jobs represent the user's desire to download something.
They are comprised of 0, 1 or many tasks.
Data fields:
1. Job ID
2. Job Status
3. List of tasks (ids)
4. Number of tasks
5. Thumbnail URL


### Tasks
Task represents single video download operation.
1. Task ID
2. Owner job ID
3. Task Status
4. Task download progress


## Browser extension
The browser extension should:
1. Work in Mozilla Firefox and Google Chrome
2. Look nice and unobtrusive in YouTube UI, as close to native UI as possible
3. Allow selecting the to download video as
4. Download single video via single click, report progress next to button
5. Download playlist via single click, report progress next to button
6. Open Web UI when clicked on in the browser's toolbar
7. If tab is closed, or video is changed, the download should not be abandoned


## Web UI
The web UI should:
1. Be lightweight, look nice to the eye, but not necessarily have fancy graphics
   1. Tailwind CSS + daisy UI
2. Show server state in real time via subscribing to server events
3. Show overall stats of server
   1. Whether there is connection to worker
   2. How to troubleshoot lack of connection, how to install worker
   3. Number of current tasks (videos)
   4. Number of current jobs (single submitted videos or playlists)
4. Show settings page, split up into server and extension parts:
   1. Server
      1. Server port
      2. Download folder
      3. Temp folder
      4. Whether to start server with the OS
      5. Number of automatic retries
      6. Number of download threads
      7. Additional yt-dlp arguments
      8. Whether to show announcements
   2. Extension
      1. Preferred download type
5. Show manual link entry prompt
6. Show queue controls:
   1. Pause all
   2. Cancel all
   3. Delete all
   4. Resume all
   5. Retry all
7. Show server kill switch
8. Show download queue entries:
   1. Thumbnail
   2. Entry name
   3. Original URL
   4. Under what type it's being downloaded
   5. Status (waiting/downloading/converting/done/failed)
   6. Show download progress (for videos, in %, for playlists, in #)
   7. For failed entries, add a button that opens raw log in a new tab
   8. For playlists, put videos in a collapsed list underneath playlist title card
9. For each entry, show control buttons:
   1. Pause button, pauses download
   2. Cancel button, cancels download
   3. Delete button, removes download from queue
   4. Resume button, resumes download
   5. Retry button, for retrying failed downloads
   6. Mark task as prioritized, mark job as prioritized
   7. For playlists, control buttons should affect all videos in playlist


## Background worker
The background worker should:
1. Work with both Windows and Linux
2. Start with the OS (optional, true by default)
3. Have low memory and CPU usage while idle
4. Keep its configuration in an external config file (any convenient format)
   1. For dev builds, config is kept in project folder
   2. For release builds, in config/ (Linux) or AppData (Windows)
   3. If config file is invalid, create new config file and rename old one
   4. Server log files should be saved to its temp folder
   5. Server log files should not blow up in size
5. Be able to download videos from YouTube (and possibly other sources) with correct format
   1. For this, yt-dlp should be used, with additional dependencies such as ffmpeg
   2. Downloads are done in temporary folder, then on success the data is moved to downloads folder
   3. On job or task error, should automatically retry multiple times (configurable)
6. Recover from interruptions, keep data after restart
   1. Temporary folder contains database with queue entries
   2. Temporary folder contains folders for each task which are then used by yt-dlp
   3. If download has been interrupted, remove temp subfolder and recreate it
   4. If database has been corrupted, rebuild the database and nuke the temp folder
   5. If downloaded video exists in 
7. Provide REST API for browser and extension:
   1. Server kill switch
   2. Statistics regarding current tasks and jobs
       1. Get stats
   3. Current server settings
       1. Get settings
       2. Modify settings
   4. Download queue
      1. Get jobs in queue
   5. Download job
      1. Get job status
      2. Create new job (automatically split it into tasks). Should create empty errored job on failure
      3. Pause job
      4. Cancel job
      5. Delete job
      6. Resume job
      7. Retry job
      8. Toggle job prioritized status
   6. Download task
      1. Get task status
      2. Pause task
      3. Cancel task
      4. Delete task
      5. Resume task
      6. Retry task
      7. Toggle task prioritized status
   7. Event stream that reports all changes in server state:
      1. Job added to queue
      2. Job removed from queue
      3. Job changed position
      4. Task added to queue
      5. Task removed from queue
      6. Task changed position
      7. Job changed status
      8. Task changed status
      9. Settings applied


TODO list:
- Common: make Linux package
- Server: automatic retries after a minute or so
- Extension: open internal webpage if no connection
- Extension: open server panel if no auth
- Extension: fix button sometimes fails to appear
- Extension: fix button carries over color to next video
- Extension: better status/error reporting
- Extension: don't update credentials from page if they are invalid
- WebUI: reduce re-creations of cards
- WebUI: better error reporting
- WebUI: split login/config/downloads into tabs
- WebUI: better config editor, fix missing features
