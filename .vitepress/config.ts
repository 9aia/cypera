import { defineConfig } from "vitepress";
import llms from "vitepress-plugin-llms";

export default defineConfig({
  title: "cypera",
  description:
    "Reusable documentation/reference generation for the 9aia org — the shared successor to the per-repo docs/gen-*.py scripts.",
  base: "/cypera/",
  head: [
    ["link", { rel: "icon", href: "/cypera/favicon.svg" }],
  ],
  themeConfig: {
    search: { provider: "local" },
    nav: [
      { text: "9aia", link: "https://9aia.com", target: "_blank" },
      { text: "Tutorials", link: "/tutorials/getting-started" },
      { text: "How-to", link: "/how-to/" },
      { text: "Explanation", link: "/explanation/" },
      { text: "Reference", link: "/reference/commands" },
      { text: "GitHub", link: "https://github.com/9aia/cypera" },
    ],
    sidebar: [
      {
        text: "Start",
        items: [{ text: "Introduction", link: "/" }],
      },
      {
        text: "Tutorials",
        items: [{ text: "Getting started", link: "/tutorials/getting-started" }],
      },
      {
        text: "How-to",
        items: [
          { text: "Generate a crate map", link: "/how-to/generate-a-crate-map" },
          { text: "Regenerate the CLI reference", link: "/how-to/cli-reference" },
          { text: "Regenerate the config reference", link: "/how-to/config-reference" },
          { text: "Check the docs contract", link: "/how-to/check-docs" },
        ],
      },
      {
        text: "Reference",
        items: [
          { text: "Commands", link: "/reference/commands" },
          { text: "Crate map", link: "/reference/crate-map" },
          { text: "cypera.toml", link: "/reference/cypera-toml" },
        ],
      },
      {
        text: "Explanation",
        items: [
          { text: "Architecture", link: "/explanation/architecture" },
          { text: "Concepts", link: "/explanation/concepts" },
          { text: "Boundaries", link: "/explanation/boundaries" },
          { text: "Decisions", link: "/explanation/decisions" },
        ],
      },
    ],
    socialLinks: [{ icon: "github", link: "https://github.com/9aia/cypera" }],
  },
  plugins: [llms()],
});
