import { mount } from 'svelte'
import '@fontsource-variable/golos-text'
import './app.css'
import App from './App.svelte'
import ReviewWindow from './lib/ReviewWindow.svelte'

// The Review window loads the same page with `?review`.
const review = new URLSearchParams(location.search).has('review')
export default mount(review ? ReviewWindow : App, { target: document.getElementById('app')! })
