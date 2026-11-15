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
    nav: [
      { text: "Guide", link: "/getting-started" },
      { text: "Reference", link: "/reference/commands" },
      { text: "GitHub", link: "https://github.com/9aia/cypera" },
    ],
    sidebar: [
      {
        text: "Start",
        items: [
          { text: "Introduction", link: "/" },
          { text: "Getting started", link: "/getting-started" },
        ],
      },
      {
        text: "Tutorials",
        items: [
          { text: "Generate a crate map", link: "/tutorials/crate-map" },
        ],
      },
      {
        text: "How-to",
        items: [
          { text: "Regenerate the CLI reference", link: "/how-to/cli-reference" },
          { text: "Regenerate the config reference", link: "/how-to/config-reference" },
        ],
      },
      {
        text: "Reference",
        items: [
          { text: "Commands", link: "/reference/commands" },
          { text: "Crate map", link: "/reference/crate-map" },
        ],
      },
      {
        text: "Explanation",
        items: [{ text: "Architecture", link: "/explanation/architecture" }],
      },
    ],
    socialLinks: [{ icon: "github", link: "https://github.com/9aia/cypera" }],
  },
  plugins: [llms()],
});
