import { setupServer } from 'msw/node'

/** The mock API; each test registers the handlers it needs with `server.use`. */
export const server = setupServer()
