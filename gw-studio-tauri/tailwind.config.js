/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,jsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ["Segoe UI", "Inter", "system-ui", "sans-serif"],
      },
      boxShadow: {
        panel: "0 0 35px rgba(0,0,0,0.30)",
      },
    },
  },
  plugins: [],
};
