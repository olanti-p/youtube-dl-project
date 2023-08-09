/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
      "webui/**/*.{html,js}",
  ],
  theme: {
    extend: {},
  },
  plugins: [
      require("daisyui"),
  ],
}

