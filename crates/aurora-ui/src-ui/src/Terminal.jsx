import { useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Terminal as XTerm } from 'xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import 'xterm/css/xterm.css'

/**
 * Terminal component using xterm.js
 * Integrates with Tauri backend for PowerShell, WSL, CMD support
 */
export function Terminal({ terminalId, shellType, onClose, onBackendIdReceived }) {
  const terminalRef = useRef(null)
  const xtermRef = useRef(null)
  const fitAddonRef = useRef(null)
  const backendIdRef = useRef(null)
  const readIntervalRef = useRef(null)

  useEffect(() => {
    if (!terminalRef.current || xtermRef.current) return

    console.log(`[Terminal] Component mounted, terminalId: ${terminalId}, shellType: ${shellType}`)

    // Create xterm instance
    const xterm = new XTerm({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: '"JetBrains Mono", Consolas, Monaco, monospace',
      theme: {
        background: 'rgba(10, 10, 20, 0.6)',
        foreground: '#E8E8E8',
        cursor: '#00FFB3',
        cursorAccent: '#1a1a2e',
        selection: 'rgba(0, 255, 179, 0.3)',
        black: '#1a1a2e',
        red: '#ff5370',
        green: '#00FFB3',
        yellow: '#ffcb6b',
        blue: '#00D9FF',
        magenta: '#B24BF3',
        cyan: '#00FFB3',
        white: '#E8E8E8',
        brightBlack: '#8892B0',
        brightRed: '#ff5370',
        brightGreen: '#00FFB3',
        brightYellow: '#ffcb6b',
        brightBlue: '#00D9FF',
        brightMagenta: '#B24BF3',
        brightCyan: '#00FFB3',
        brightWhite: '#ffffff',
      },
      allowProposedApi: true,
    })

    // Add fit addon for responsive sizing
    const fitAddon = new FitAddon()
    xterm.loadAddon(fitAddon)

    // Add web links addon
    xterm.loadAddon(new WebLinksAddon())

    // Open terminal in DOM
    xterm.open(terminalRef.current)
    fitAddon.fit()

    // Store refs
    xtermRef.current = xterm
    fitAddonRef.current = fitAddon

    let resizeObserver = null

    // Spawn terminal on backend
    const cols = xterm.cols
    const rows = xterm.rows

    console.log(`[Terminal] Spawning terminal with shell ${shellType}, cols=${cols}, rows=${rows}`)

    invoke('spawn_terminal', { shellType, cols, rows })
      .then(async (id) => {
        console.log(`[Terminal] Backend returned terminal ID: ${id}`)
        backendIdRef.current = id
        if (onBackendIdReceived) {
          onBackendIdReceived(id)
        }

        // Write a visual indicator that we're setting up listeners
        xterm.write(`\x1b[36m[Setting up terminal listeners for ${id}...]\x1b[0m\r\n`)

        // Listen for terminal output events
        console.log(`[Terminal] Setting up output listener for: terminal-${id}-output`)
        const outputUnlisten = await listen(`terminal-${id}-output`, (event) => {
          console.log(`[Terminal] ${id} received output event, payload length:`, event.payload?.length, 'first 50 chars:', event.payload?.substring(0, 50))
          if (event.payload && event.payload.length > 0) {
            xterm.write(event.payload)
          }
        })
        console.log(`[Terminal] Output listener registered for ${id}`)

        // Listen for terminal close events
        console.log(`[Terminal] Setting up close listener for: terminal-${id}-closed`)
        const closeUnlisten = await listen(`terminal-${id}-closed`, () => {
          console.log(`[Terminal] ${id} closed by backend`)
          if (onClose) {
            onClose()
          }
        })

        // Listen for terminal error events
        console.log(`[Terminal] Setting up error listener for: terminal-${id}-error`)
        const errorUnlisten = await listen(`terminal-${id}-error`, (event) => {
          console.error(`[Terminal] ${id} error:`, event.payload)
          xterm.write(`\r\n\x1b[31mTerminal error: ${event.payload}\x1b[0m\r\n`)
        })

        console.log(`[Terminal] All listeners registered for ${id}`)
        xterm.write(`\x1b[32m[Terminal listeners ready]\x1b[0m\r\n`)

        // Store unlisten functions
        readIntervalRef.current = { outputUnlisten, closeUnlisten, errorUnlisten }

        // Handle resize
        const handleResize = () => {
          if (fitAddon && xterm && id) {
            fitAddon.fit()
            invoke('resize_terminal', {
              id,
              cols: xterm.cols,
              rows: xterm.rows,
            }).catch((err) => {
              console.error('Failed to resize terminal:', err)
            })
          }
        }

        // Attach resize observer
        resizeObserver = new ResizeObserver(handleResize)
        if (terminalRef.current) {
          resizeObserver.observe(terminalRef.current)
        }
      })
      .catch((err) => {
        console.error('[Terminal] Failed to spawn terminal:', err)
        xterm.write(`\r\n\x1b[31mFailed to spawn terminal: ${err}\x1b[0m\r\n`)
      })

    // Handle user input - set up immediately
    const dataHandler = xterm.onData((data) => {
      if (backendIdRef.current) {
        console.log(`[Terminal] Writing ${data.length} bytes to ${backendIdRef.current}`)
        invoke('write_terminal', { id: backendIdRef.current, data }).catch((err) => {
          console.error('[Terminal] Failed to write to terminal:', err)
        })
      } else {
        console.warn('[Terminal] Received input but backend ID not yet available')
      }
    })

    // Cleanup
    return () => {
      if (readIntervalRef.current) {
        // Unlisten from all events
        const { outputUnlisten, closeUnlisten, errorUnlisten } = readIntervalRef.current
        if (outputUnlisten) outputUnlisten()
        if (closeUnlisten) closeUnlisten()
        if (errorUnlisten) errorUnlisten()
      }
      if (resizeObserver) {
        resizeObserver.disconnect()
      }
      if (dataHandler) {
        dataHandler.dispose()
      }
      if (backendIdRef.current) {
        invoke('close_terminal', { id: backendIdRef.current }).catch((err) => {
          console.error('Failed to close terminal:', err)
        })
      }
      if (xtermRef.current) {
        xtermRef.current.dispose()
      }
    }
  }, [terminalId, shellType])

  return (
    <div className="h-full w-full">
      <div ref={terminalRef} className="h-full w-full" />
    </div>
  )
}

/**
 * Terminal panel with multi-tab support
 */
export function TerminalPanel() {
  const [terminals, setTerminals] = useState([])
  const [activeTerminal, setActiveTerminal] = useState(null)
  const [availableShells, setAvailableShells] = useState([])
  const [defaultShell, setDefaultShell] = useState(null)
  const [isCollapsed, setIsCollapsed] = useState(false)
  const [showShellSelector, setShowShellSelector] = useState(false)
  const dropdownRef = useRef(null)

  // Load available shells on mount
  useEffect(() => {
    invoke('get_available_shells')
      .then((shells) => {
        setAvailableShells(shells)
      })
      .catch((err) => {
        console.error('Failed to get available shells:', err)
      })

    invoke('get_default_shell')
      .then((shell) => {
        setDefaultShell(shell)
      })
      .catch((err) => {
        console.error('Failed to get default shell:', err)
      })
  }, [])

  // Click outside handler to close dropdown
  useEffect(() => {
    const handleClickOutside = (event) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target)) {
        setShowShellSelector(false)
      }
    }

    if (showShellSelector) {
      document.addEventListener('mousedown', handleClickOutside)
      return () => document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [showShellSelector])

  const spawnTerminal = (shellType) => {
    const id = `term-${Date.now()}`
    const newTerm = {
      id,
      shellType: shellType || defaultShell,
      title: `${shellType || defaultShell} - ${terminals.length + 1}`,
    }
    setTerminals([...terminals, newTerm])
    setActiveTerminal(id)
    setShowShellSelector(false)
  }

  const closeTerminal = (id) => {
    setTerminals(terminals.filter((t) => t.id !== id))
    if (activeTerminal === id) {
      const remaining = terminals.filter((t) => t.id !== id)
      setActiveTerminal(remaining.length > 0 ? remaining[0].id : null)
    }
  }

  const activeTerm = terminals.find((t) => t.id === activeTerminal)

  return (
    <div className="flex flex-col h-full">
      {/* Terminal tabs header */}
      <div className="flex items-center gap-2 px-3 py-2 bg-gradient-to-r from-purple/10 to-blue/10 backdrop-blur-md border-b border-white/10">
        {/* Collapse/Expand button */}
        <button
          onClick={() => setIsCollapsed(!isCollapsed)}
          className="px-2 py-1 rounded hover:bg-white/5 transition-colors text-xs text-text-dim"
          title={isCollapsed ? 'Expand Terminal' : 'Collapse Terminal'}
        >
          {isCollapsed ? '▲' : '▼'}
        </button>

        <span className="text-sm font-semibold text-text-primary">Terminal</span>

        {/* Terminal tabs */}
        <div className="flex-1 flex items-center gap-1 overflow-x-auto">
          {terminals.map((term) => (
            <div
              key={term.id}
              className={`flex items-center gap-2 px-3 py-1 rounded cursor-pointer transition-colors ${
                activeTerminal === term.id
                  ? 'bg-glacial-blue/20 border border-glacial-blue/30'
                  : 'bg-white/5 hover:bg-white/10'
              }`}
              onClick={() => setActiveTerminal(term.id)}
            >
              <span className="text-xs">{term.title}</span>
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  closeTerminal(term.id)
                }}
                className="text-text-dim hover:text-red-400 transition-colors"
              >
                ×
              </button>
            </div>
          ))}
        </div>

        {/* New terminal dropdown */}
        <div className="relative" ref={dropdownRef}>
          <div className="flex items-center gap-1">
            <button
              onClick={() => spawnTerminal(defaultShell)}
              className="px-3 py-1 rounded-l bg-glacial-blue/20 hover:bg-glacial-blue/30 transition-colors text-sm"
              title="New Terminal"
            >
              + New
            </button>
            {availableShells.length > 1 && (
              <button
                onClick={() => setShowShellSelector(!showShellSelector)}
                className="px-2 py-1 rounded-r bg-glacial-blue/20 hover:bg-glacial-blue/30 transition-colors text-sm border-l border-white/10"
                title="Select Shell"
              >
                ▼
              </button>
            )}
          </div>
          {availableShells.length > 1 && showShellSelector && (
            <div className="absolute right-0 mt-1 bg-panel-bg/95 backdrop-blur-md border border-white/10 rounded-lg shadow-xl z-50 min-w-[150px]">
              {availableShells.map((shell) => (
                <button
                  key={shell}
                  onClick={(e) => {
                    e.preventDefault()
                    e.stopPropagation()
                    spawnTerminal(shell)
                  }}
                  className="w-full text-left px-3 py-2 hover:bg-white/5 transition-colors text-sm first:rounded-t-lg last:rounded-b-lg"
                >
                  {shell}
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Terminal content */}
      {!isCollapsed && (
        <div className="flex-1 relative">
          {terminals.length === 0 ? (
            <div className="flex items-center justify-center h-full text-text-dim">
              <div className="text-center">
                <p className="text-sm mb-2">No terminal sessions</p>
                <button
                  onClick={() => spawnTerminal(defaultShell)}
                  className="px-4 py-2 rounded bg-glacial-blue/20 hover:bg-glacial-blue/30 transition-colors text-sm"
                >
                  Start Terminal
                </button>
              </div>
            </div>
          ) : (
            terminals.map((term) => (
              <div
                key={term.id}
                className={`absolute inset-0 ${activeTerminal === term.id ? 'block' : 'hidden'}`}
              >
                <Terminal
                  terminalId={term.id}
                  shellType={term.shellType}
                  onClose={() => closeTerminal(term.id)}
                />
              </div>
            ))
          )}
        </div>
      )}
    </div>
  )
}
