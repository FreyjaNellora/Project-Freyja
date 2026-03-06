// Analysis panel: depth, scores, nodes, NPS, PV.

import type { InfoData } from '../types/protocol';

interface AnalysisPanelProps {
  latestInfo: InfoData | null;
}

export default function AnalysisPanel({ latestInfo }: AnalysisPanelProps) {
  if (!latestInfo) {
    return (
      <div className="analysis-panel">
        <h3>Analysis</h3>
        <div className="analysis-empty">No search data</div>
      </div>
    );
  }

  return (
    <div className="analysis-panel">
      <h3>Analysis</h3>
      <div className="analysis-grid">
        {latestInfo.depth != null && (
          <div className="analysis-item">
            <span className="analysis-label">Depth</span>
            <span className="analysis-value">{latestInfo.depth}</span>
          </div>
        )}
        {latestInfo.nodes != null && (
          <div className="analysis-item">
            <span className="analysis-label">Nodes</span>
            <span className="analysis-value">{latestInfo.nodes.toLocaleString()}</span>
          </div>
        )}
        {latestInfo.nps != null && (
          <div className="analysis-item">
            <span className="analysis-label">NPS</span>
            <span className="analysis-value">{latestInfo.nps.toLocaleString()}</span>
          </div>
        )}
      </div>
      {latestInfo.scores && (
        <div className="analysis-scores">
          <span className="analysis-label">Scores</span>
          <span className="score-red">R:{latestInfo.scores[0]}</span>
          <span className="score-blue">B:{latestInfo.scores[1]}</span>
          <span className="score-yellow">Y:{latestInfo.scores[2]}</span>
          <span className="score-green">G:{latestInfo.scores[3]}</span>
        </div>
      )}
      {latestInfo.pv && latestInfo.pv.length > 0 && (
        <div className="analysis-pv">
          <span className="analysis-label">PV</span>
          <span className="pv-line">{latestInfo.pv.join(' ')}</span>
        </div>
      )}
    </div>
  );
}
