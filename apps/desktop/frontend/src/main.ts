import { mount } from 'svelte'

import App from './App.svelte'
import './styles/global.css'

const target = document.getElementById('app')

if (!target) {
  throw new Error('Nanika frontend mount point is missing')
}

mount(App, { target })
