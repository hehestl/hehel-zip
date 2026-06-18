/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "Manrope",
          "Segoe UI Emoji",
          "Segoe UI Symbol",
          "system-ui",
          "sans-serif",
        ],
      },
      colors: {
        hh: {
          bg: "#121212",
          surface: "#151515",
          accent: "#2659FF",
          link: "#578CFF",
          border: "#2A2A2A",
          danger: "#FF4D4D",
        },
        win: {
          bg: "#121212",
          panel: "#151515",
          border: "#2A2A2A",
          header: "#151515",
          select: "#2659FF",
        },
      },
      borderRadius: {
        "hh-sm": "10px",
        "hh-lg": "15px",
      },
    },
  },
  plugins: [],
};
