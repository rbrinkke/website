/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./templates/**/*.html"],
  corePlugins: {
    preflight: false,
  },
  theme: {
    extend: {
      colors: {
        goamet: {
          blue: "#1E88E5",
          pink: "#FF0066",
          navy: "#0B1220",
          bg: "#F3F4F6",
        },
      },
      boxShadow: {
        glow: "0 20px 60px rgba(30,136,229,.18), 0 8px 24px rgba(255,0,102,.10)",
      },
    },
  },
};

