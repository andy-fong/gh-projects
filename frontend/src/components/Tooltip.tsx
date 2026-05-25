import { useState } from 'react'
import { createPortal } from 'react-dom'

interface Props {
  text: string
  children: React.ReactNode
}

export default function Tooltip({ text, children }: Props) {
  const [rect, setRect] = useState<DOMRect | null>(null)

  if (!text) return <>{children}</>

  return (
    <span
      className="block truncate"
      onMouseEnter={e => setRect((e.currentTarget as HTMLElement).getBoundingClientRect())}
      onMouseLeave={() => setRect(null)}
    >
      {children}
      {rect && createPortal(
        <div
          className="fixed z-[9999] px-2 py-1.5 text-xs bg-gray-900 border border-gray-600 rounded shadow-xl text-gray-100 max-w-sm break-words pointer-events-none whitespace-pre-wrap"
          style={{
            left: rect.left,
            top: rect.top - 6,
            transform: 'translateY(-100%)',
          }}
        >
          {text}
        </div>,
        document.body
      )}
    </span>
  )
}
