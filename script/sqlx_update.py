import os

# Update sqlx "offline" mode file with contents of our debug database.
#
# Because sqlx only understands the current version of the database,
# the usual workflow is:
#    1. Figure out how you would want to update the database
#    2. Write a new migration with `cargo sqlx migrate add my_new_migration`
#    3. Compile and run, to have migration be applied to the database
#    4. Start writing your code
#    5. Run this script to update "offline mode" files with new version of db
#    6. Compile the project as usual (e.g. with "cargo build")

os.system("cargo sqlx prepare --database-url \"sqlite://debug/db/state.db\"")
