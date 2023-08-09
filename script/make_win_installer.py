import os

# First, update our CSS
os.system("npx tailwindcss -i ./webui_template/index.css -o ./webui/index.css")
# Next, build our build
os.system("cargo build --release")
# And now, we can make an installer
os.system("\"C:\Program Files (x86)\Inno Setup 6\ISCC.exe\" installer\windows\main.iss")
