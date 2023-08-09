import os

# Run tailwind in "watch" mode
os.system("npx tailwindcss -i ./webui_template/index.css -o ./webui/index.css --watch")
