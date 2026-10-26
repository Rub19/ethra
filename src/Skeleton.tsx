export default function Skeleton({ rows = 6, height = 76 }: { rows?: number; height?: number }) {
  return (
    <div className="skel" aria-busy="true" aria-label="Chargement">
      {Array.from({ length: rows }, (_, i) => <div key={i} style={{ height }} />)}
    </div>
  );
}
