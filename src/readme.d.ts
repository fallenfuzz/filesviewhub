/** README.md as HTML, made by vite.config.ts at build time. */
declare module 'virtual:readme' {
  const html: string
  export default html
  export const chapters: { id: string; text: string }[]
}
