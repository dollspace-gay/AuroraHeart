import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism'

// Custom Aurora syntax theme
const auroraTheme = {
  ...vscDarkPlus,
  'code[class*="language-"]': {
    ...vscDarkPlus['code[class*="language-"]'],
    background: 'transparent',
    color: '#E8E8E8',
    fontFamily: '"JetBrains Mono", Consolas, Monaco, monospace',
  },
  'pre[class*="language-"]': {
    ...vscDarkPlus['pre[class*="language-"]'],
    background: 'transparent',
    margin: 0,
    padding: '1rem',
  },
  comment: { color: '#8892B0' },
  prolog: { color: '#8892B0' },
  doctype: { color: '#8892B0' },
  cdata: { color: '#8892B0' },
  punctuation: { color: '#E8E8E8' },
  property: { color: '#00FFB3' },
  tag: { color: '#B24BF3' },
  boolean: { color: '#00D9FF' },
  number: { color: '#00D9FF' },
  'function-name': { color: '#00FF9D' },
  constant: { color: '#00D9FF' },
  symbol: { color: '#00D9FF' },
  deleted: { color: '#ff5370' },
  selector: { color: '#B24BF3' },
  'attr-name': { color: '#00FFB3' },
  string: { color: '#00FFB3' },
  char: { color: '#00FFB3' },
  builtin: { color: '#B24BF3' },
  inserted: { color: '#00FFB3' },
  operator: { color: '#E8E8E8' },
  entity: { color: '#00D9FF' },
  url: { color: '#00FFB3' },
  variable: { color: '#00FFB3' },
  atrule: { color: '#B24BF3' },
  'attr-value': { color: '#00FFB3' },
  keyword: { color: '#B24BF3' },
  regex: { color: '#00FFB3' },
  important: { color: '#B24BF3', fontWeight: 'bold' },
  function: { color: '#00FF9D' },
  'class-name': { color: '#00D9FF' },
}

// File tree item component with expansion support
function FileTreeItem({ item, level = 0, onFileClick, currentFile, expandedFolders, onToggleFolder }) {
  const isExpanded = expandedFolders.has(item.path)
  const isSelected = currentFile === item.path

  const handleClick = () => {
    if (item.is_directory) {
      onToggleFolder(item)
    } else {
      onFileClick(item)
    }
  }

  return (
    <div>
      <div
        onClick={handleClick}
        className={`px-3 py-2 rounded hover:bg-white/5 cursor-pointer transition-colors flex items-center gap-2 ${
          isSelected && !item.is_directory ? 'bg-glacial-blue/10 border-l-2 border-glacial-blue' : ''
        }`}
        style={{ paddingLeft: `${level * 12 + 12}px` }}
      >
        {item.is_directory && (
          <span className="text-xs text-text-dim">{isExpanded ? '▼' : '▶'}</span>
        )}
        <span className="text-sm">{item.is_directory ? '📁' : '📄'} {item.name}</span>
      </div>
      {item.is_directory && isExpanded && item.children && (
        <div>
          {item.children.map((child, index) => (
            <FileTreeItem
              key={index}
              item={child}
              level={level + 1}
              onFileClick={onFileClick}
              currentFile={currentFile}
              expandedFolders={expandedFolders}
              onToggleFolder={onToggleFolder}
            />
          ))}
        </div>
      )}
    </div>
  )
}

function App() {
  const [currentFile, setCurrentFile] = useState('')
  const [editorText, setEditorText] = useState('')
  const [chatInput, setChatInput] = useState('')
  const [chatOutput, setChatOutput] = useState('')
  const [isModified, setIsModified] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [apiKey, setApiKey] = useState('')
  const [fileTree, setFileTree] = useState([])
  const [expandedFolders, setExpandedFolders] = useState(new Set())
  const [isEditing, setIsEditing] = useState(false)

  // Load file tree on mount
  useEffect(() => {
    loadFileTree()
  }, [])

  const loadFileTree = async () => {
    try {
      const files = await invoke('get_file_tree')
      setFileTree(files)
    } catch (error) {
      console.error('Failed to load file tree:', error)
    }
  }

  const handleToggleFolder = async (folder) => {
    const newExpanded = new Set(expandedFolders)

    if (newExpanded.has(folder.path)) {
      newExpanded.delete(folder.path)
      setExpandedFolders(newExpanded)
    } else {
      newExpanded.add(folder.path)
      setExpandedFolders(newExpanded)

      // Load folder contents if not already loaded
      if (!folder.children) {
        try {
          const children = await invoke('get_directory_contents', { path: folder.path })
          // Update the file tree with loaded children
          const updateTreeWithChildren = (items) => {
            return items.map(item => {
              if (item.path === folder.path) {
                return { ...item, children }
              } else if (item.children) {
                return { ...item, children: updateTreeWithChildren(item.children) }
              }
              return item
            })
          }
          setFileTree(updateTreeWithChildren(fileTree))
        } catch (error) {
          console.error('Failed to load folder contents:', error)
        }
      }
    }
  }

  const handleOpenFile = async () => {
    try {
      const result = await invoke('open_file')
      if (result) {
        setCurrentFile(result.path)
        setEditorText(result.content)
        setIsModified(false)
        setIsEditing(false)
      }
    } catch (error) {
      console.error('Failed to open file:', error)
    }
  }

  const handleSaveFile = async () => {
    try {
      await invoke('save_file', { path: currentFile, content: editorText })
      setIsModified(false)
    } catch (error) {
      console.error('Failed to save file:', error)
    }
  }

  const handleSendMessage = async () => {
    if (!chatInput.trim()) return

    const userMessage = chatInput
    setChatInput('')
    setChatOutput(prev => prev + `\n\nYou: ${userMessage}\n\nClaude: `)

    try {
      const response = await invoke('send_message', { message: userMessage })
      setChatOutput(prev => prev + response)
    } catch (error) {
      setChatOutput(prev => prev + `Error: ${error}`)
    }
  }

  const handleSaveApiKey = async () => {
    try {
      await invoke('save_api_key', { key: apiKey })
      setShowSettings(false)
      setApiKey('')
    } catch (error) {
      console.error('Failed to save API key:', error)
    }
  }

  const handleFileClick = async (item) => {
    if (item.is_directory) return

    try {
      const content = await invoke('read_file_by_path', { path: item.path })
      setCurrentFile(item.path)
      setEditorText(content)
      setIsModified(false)
      setIsEditing(false)
    } catch (error) {
      console.error('Failed to open file:', error)
    }
  }

  // Detect language from file extension
  const getLanguage = (filename) => {
    const ext = filename.split('.').pop().toLowerCase()
    const langMap = {
      js: 'javascript',
      jsx: 'jsx',
      ts: 'typescript',
      tsx: 'tsx',
      rs: 'rust',
      py: 'python',
      go: 'go',
      java: 'java',
      cpp: 'cpp',
      c: 'c',
      h: 'c',
      hpp: 'cpp',
      cs: 'csharp',
      html: 'html',
      css: 'css',
      scss: 'scss',
      json: 'json',
      xml: 'xml',
      md: 'markdown',
      sh: 'bash',
      yml: 'yaml',
      yaml: 'yaml',
      toml: 'toml',
      sql: 'sql',
      rb: 'ruby',
      php: 'php',
    }
    return langMap[ext] || 'text'
  }

  return (
    <div className="w-full h-full relative">
      {/* Aurora Background Blobs */}
      <div className="aurora-blob aurora-blob-1" />
      <div className="aurora-blob aurora-blob-2" />
      <div className="aurora-blob aurora-blob-3" />
      <div className="aurora-blob aurora-blob-4" />
      <div className="aurora-blob aurora-blob-5" />
      <div className="aurora-blob aurora-blob-6" />

      {/* Main Layout */}
      <div className="w-full h-full flex flex-col">
        {/* Glassmorphic Toolbar */}
        <div className="glass-panel h-12 flex items-center px-6 gap-6 relative">
          {/* Top aurora glow */}
          <div
            className="absolute top-0 left-0 right-0 h-0.5 opacity-60"
            style={{
              background: 'linear-gradient(90deg, transparent 0%, rgba(0, 255, 179, 0.6) 25%, rgba(0, 217, 255, 0.6) 50%, rgba(178, 75, 243, 0.6) 75%, transparent 100%)'
            }}
          />

          {/* Logo */}
          <div className="flex items-center gap-3">
            <span className="text-aurora-green text-xl">◆</span>
            <span className="text-aurora-green text-lg font-extrabold">AURORA</span>
            <span className="text-glacial-blue text-lg font-extrabold">HEART</span>
          </div>

          {/* Buttons */}
          <div className="flex gap-3">
            <button
              onClick={handleOpenFile}
              className="glass-panel px-4 py-2 text-sm rounded-md hover:glow-green transition-glow"
            >
              Open
            </button>
            <button
              onClick={handleSaveFile}
              disabled={!currentFile || !isModified}
              className={`glass-panel px-4 py-2 text-sm rounded-md transition-glow ${
                isModified ? 'bg-aurora-green text-deep-space glow-green' : 'opacity-50'
              }`}
            >
              Save
            </button>
            <button
              onClick={() => setShowSettings(true)}
              className="glass-panel px-4 py-2 text-sm rounded-md hover:glow-blue transition-glow"
            >
              Settings
            </button>
            <button
              onClick={() => setChatOutput('')}
              className="glass-panel px-4 py-2 text-sm rounded-md hover:glow-purple transition-glow"
            >
              Clear Chat
            </button>
          </div>
        </div>

        {/* Main Content */}
        <div className="flex-1 flex overflow-hidden">
          {/* Glassmorphic Sidebar */}
          <div className="glass-panel w-70 flex flex-col border-r border-white/10">
            <div className="glass-panel-light h-11 flex items-center px-4 border-b border-white/10">
              <span className="text-xs">📁</span>
              <span className="text-xs font-bold tracking-wider ml-2 text-text-dim">PROJECT</span>
            </div>
            <div className="flex-1 overflow-auto p-3">
              {fileTree.length > 0 ? (
                <div className="space-y-1">
                  {fileTree.map((item, index) => (
                    <FileTreeItem
                      key={index}
                      item={item}
                      onFileClick={handleFileClick}
                      currentFile={currentFile}
                      expandedFolders={expandedFolders}
                      onToggleFolder={handleToggleFolder}
                    />
                  ))}
                </div>
              ) : (
                <div className="text-center py-8">
                  <p className="text-text-dim text-sm">No Files</p>
                  <p className="text-text-dim text-xs mt-2">Open a file to begin</p>
                </div>
              )}
            </div>
          </div>

          {/* Editor Area */}
          <div className="flex-1 flex flex-col">
            {/* Editor Tab */}
            {currentFile && (
              <div className="glass-panel h-11 flex items-center px-4 gap-2 border-b border-white/10">
                {isModified && (
                  <div className="w-2.5 h-2.5 rounded-full bg-aurora-green glow-green" />
                )}
                <span className="text-sm">{currentFile.split(/[\\/]/).pop()}</span>
                {!isEditing && (
                  <button
                    onClick={() => setIsEditing(true)}
                    className="ml-auto text-xs glass-panel px-3 py-1 rounded hover:glow-blue transition-glow"
                  >
                    Edit
                  </button>
                )}
              </div>
            )}

            {/* Editor Content */}
            <div className="flex-1 bg-deep-space overflow-hidden">
              {currentFile || editorText ? (
                <div className="h-full overflow-auto">
                  {isEditing ? (
                    <div className="flex h-full">
                      {/* Line Numbers */}
                      <div className="glass-panel w-16 border-r border-white/10 p-4">
                        <div className="text-right text-text-dim text-xs font-mono space-y-1">
                          {editorText.split('\n').map((_, i) => (
                            <div key={i} style={{ lineHeight: '1.5' }}>{i + 1}</div>
                          ))}
                        </div>
                      </div>
                      {/* Editor */}
                      <textarea
                        value={editorText}
                        onChange={(e) => {
                          setEditorText(e.target.value)
                          setIsModified(true)
                        }}
                        className="flex-1 bg-transparent text-text-white font-mono text-sm p-4 outline-none resize-none"
                        style={{ caretColor: '#00FFB3', lineHeight: '1.5' }}
                      />
                    </div>
                  ) : (
                    <SyntaxHighlighter
                      language={getLanguage(currentFile)}
                      style={auroraTheme}
                      showLineNumbers={true}
                      lineNumberStyle={{ color: '#8892B0', minWidth: '3em', paddingRight: '1em' }}
                      customStyle={{
                        margin: 0,
                        background: 'transparent',
                        fontSize: '0.875rem',
                        lineHeight: '1.5',
                      }}
                    >
                      {editorText}
                    </SyntaxHighlighter>
                  )}
                </div>
              ) : (
                /* Welcome Screen */
                <div className="flex flex-col items-center justify-center h-full gap-8">
                  {/* Aurora Glow */}
                  <div
                    className="w-75 h-75 rounded-full opacity-30"
                    style={{
                      background: 'radial-gradient(circle, rgba(0, 255, 179, 0.5) 0%, rgba(0, 217, 255, 0.5) 40%, transparent 70%)'
                    }}
                  />

                  {/* Logo */}
                  <div className="flex flex-col items-center gap-4">
                    <div className="flex items-center gap-3">
                      <span className="text-aurora-green text-3xl">◆</span>
                      <span className="text-aurora-green text-2xl font-extrabold">AURORA HEART</span>
                    </div>
                    <p className="text-text-dim text-sm font-light">Glassmorphic AI-Powered Development</p>
                  </div>

                  {/* Syntax Colors Showcase */}
                  <div className="glass-panel rounded-xl border border-glacial-blue/25 p-6 w-125">
                    <div className="flex items-center gap-2 mb-4">
                      <span>✨</span>
                      <span className="text-xs font-bold tracking-wider text-text-dim">AURORA SYNTAX COLORS</span>
                    </div>
                    <div className="font-mono text-sm space-y-2">
                      <div className="text-text-dim">// Syntax highlighting with Aurora colors</div>
                      <div className="flex gap-2">
                        <span className="text-nebula-purple font-bold">fn</span>
                        <span className="text-aurora-green-alt">calculate</span>
                        <span>(value:</span>
                        <span className="text-glacial-blue font-semibold">i32</span>
                        <span>) {'{'}</span>
                      </div>
                      <div className="pl-4 flex gap-2">
                        <span className="text-nebula-purple font-bold">let</span>
                        <span>result</span>
                        <span>=</span>
                        <span className="text-glacial-blue font-semibold">42</span>
                        <span>;</span>
                      </div>
                      <div className="pl-4 flex gap-2">
                        <span className="text-aurora-green-alt">println!</span>
                        <span>(</span>
                        <span className="text-aurora-green">"Aurora Borealis"</span>
                        <span>);</span>
                      </div>
                      <div>{'}'}</div>
                      <div className="mt-4 grid grid-cols-2 gap-3 text-xs">
                        <div className="flex items-center gap-2">
                          <div className="w-4 h-4 rounded bg-nebula-purple" />
                          <span className="text-text-dim">Keywords</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <div className="w-4 h-4 rounded bg-aurora-green" />
                          <span className="text-text-dim">Strings</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <div className="w-4 h-4 rounded bg-aurora-green-alt" />
                          <span className="text-text-dim">Functions</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <div className="w-4 h-4 rounded bg-glacial-blue" />
                          <span className="text-text-dim">Numbers</span>
                        </div>
                      </div>
                    </div>
                  </div>

                  <p className="text-text-dim text-sm">Open a file or ask the AI assistant</p>
                </div>
              )}
            </div>
          </div>

          {/* Glassmorphic Chat Panel */}
          <div className="glass-panel w-105 flex flex-col border-l border-glacial-blue/25">
            <div className="glass-panel-light h-11 flex items-center px-4 gap-2 border-b border-white/10">
              <span>🤖</span>
              <span className="text-xs font-bold tracking-wider text-glacial-blue">AI ASSISTANT</span>
            </div>

            {/* Chat Messages */}
            <div className="flex-1 overflow-auto p-4">
              <pre className="text-sm whitespace-pre-wrap font-mono">{chatOutput}</pre>
            </div>

            {/* Chat Input */}
            <div className="glass-panel-light h-35 border-t border-glacial-blue/25 p-4 flex flex-col gap-3">
              <textarea
                value={chatInput}
                onChange={(e) => setChatInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && e.ctrlKey) {
                    handleSendMessage()
                  }
                }}
                placeholder="Ask Claude for help..."
                className="glass-panel flex-1 rounded-lg p-3 text-sm outline-none resize-none text-text-white"
                style={{ caretColor: '#00FFB3' }}
              />
              <div className="flex justify-end">
                <button
                  onClick={handleSendMessage}
                  disabled={!chatInput.trim()}
                  className={`px-4 py-2 rounded-md text-sm font-semibold transition-glow ${
                    chatInput.trim()
                      ? 'bg-aurora-green text-deep-space glow-green'
                      : 'glass-panel opacity-50'
                  }`}
                >
                  Send
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* Glassmorphic Status Bar */}
        <div className="glass-panel h-7 flex items-center px-6 gap-4 border-t border-white/10 relative">
          <div
            className="absolute bottom-0 left-0 right-0 h-0.5 opacity-50"
            style={{
              background: 'linear-gradient(90deg, transparent 0%, rgba(0, 217, 255, 0.5) 50%, transparent 100%)'
            }}
          />
          <div className="flex items-center gap-2">
            <span className="text-aurora-green text-xs">◆</span>
            <span className="text-aurora-green text-xs font-semibold">AuroraHeart</span>
          </div>
          <div className="w-px h-4 bg-white/10" />
          <span className="text-text-dim text-xs">{isEditing ? 'Editing' : 'Ready'}</span>
        </div>
      </div>

      {/* Settings Dialog */}
      {showSettings && (
        <div className="absolute inset-0 flex items-center justify-center" style={{ background: 'rgba(11, 12, 21, 0.8)' }}>
          <div className="glass-panel rounded-2xl p-8 w-160 border border-glacial-blue/25 glow-blue">
            <div className="flex items-center justify-between mb-6">
              <div className="flex items-center gap-3">
                <span className="text-2xl">⚙️</span>
                <h2 className="text-xl font-bold">Settings</h2>
              </div>
              <button
                onClick={() => setShowSettings(false)}
                className="glass-panel w-10 h-10 rounded-lg flex items-center justify-center hover:bg-white/10 transition-colors"
              >
                <span className="text-lg">✕</span>
              </button>
            </div>

            <div className="h-px bg-white/10 mb-6" />

            <div className="space-y-4">
              <div className="flex items-center gap-2">
                <span>🔑</span>
                <label className="text-sm font-bold">Anthropic API Key</label>
              </div>
              <p className="text-text-dim text-xs">Your API key is encrypted and stored securely on your device.</p>
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="sk-ant-..."
                className="glass-panel w-full rounded-lg p-3 text-sm outline-none text-text-white"
                style={{ caretColor: '#00FFB3' }}
              />
            </div>

            <div className="flex justify-end gap-3 mt-8">
              <button
                onClick={() => setShowSettings(false)}
                className="glass-panel px-6 py-2 rounded-md text-sm hover:bg-white/10 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleSaveApiKey}
                disabled={!apiKey}
                className={`px-6 py-2 rounded-md text-sm font-semibold transition-glow ${
                  apiKey ? 'bg-aurora-green text-deep-space glow-green' : 'glass-panel opacity-50'
                }`}
              >
                Save Key
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

export default App
