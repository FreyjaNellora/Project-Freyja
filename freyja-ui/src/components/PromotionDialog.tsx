// Pawn promotion piece selector modal.
// Shows 4 buttons (Queen, Rook, Bishop, Knight).
// Escape or backdrop click cancels.

import { useEffect } from 'react';

interface PromotionDialogProps {
  onSelect: (piece: 'q' | 'r' | 'b' | 'n') => void;
  onCancel: () => void;
}

export default function PromotionDialog({ onSelect, onCancel }: PromotionDialogProps) {
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onCancel();
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [onCancel]);

  const options: { label: string; symbol: string; value: 'q' | 'r' | 'b' | 'n' }[] = [
    { label: 'Queen', symbol: '\u2655', value: 'q' },
    { label: 'Rook', symbol: '\u2656', value: 'r' },
    { label: 'Bishop', symbol: '\u2657', value: 'b' },
    { label: 'Knight', symbol: '\u2658', value: 'n' },
  ];

  return (
    <div className="promotion-overlay" onClick={onCancel}>
      <div className="promotion-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="promotion-title">Promote to:</div>
        <div className="promotion-options">
          {options.map((opt) => (
            <button
              key={opt.value}
              className="promotion-btn"
              onClick={() => onSelect(opt.value)}
              title={opt.label}
            >
              <span className="promotion-symbol">{opt.symbol}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
