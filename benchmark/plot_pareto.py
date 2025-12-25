#!/usr/bin/env python3
"""
Generate Pareto front plots from benchmark results.

Usage:
    python plot_pareto.py benchmark_results.csv

Outputs:
    - pareto_ssimulacra2.svg: SSIMULACRA2 vs BPP
    - pareto_butteraugli.svg: Butteraugli vs BPP
    - pareto_dssim.svg: DSSIM vs BPP

All charts support both light and dark mode via CSS media queries.
"""

import sys
import csv
from collections import defaultdict

def load_csv(path):
    """Load benchmark results CSV."""
    results = []
    with open(path, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            result = {
                'corpus': row['corpus'],
                'image': row['image'],
                'encoder': row['encoder'],
                'quality': int(row['quality']),
                'file_size': int(row['file_size']),
                'bpp': float(row['bpp']),
                'ssimulacra2': float(row['ssimulacra2']),
                'dssim': float(row['dssim']),
            }
            # Butteraugli is optional (for backwards compatibility)
            if 'butteraugli' in row:
                result['butteraugli'] = float(row['butteraugli'])
            results.append(result)
    return results

def aggregate_by_quality(results):
    """Aggregate results by encoder and quality level."""
    agg = defaultdict(lambda: defaultdict(list))

    for r in results:
        key = (r['encoder'], r['quality'])
        agg[key]['bpp'].append(r['bpp'])
        agg[key]['ssimulacra2'].append(r['ssimulacra2'])
        agg[key]['dssim'].append(r['dssim'])
        if 'butteraugli' in r:
            agg[key]['butteraugli'].append(r['butteraugli'])

    aggregated = []
    for (encoder, quality), values in agg.items():
        item = {
            'encoder': encoder,
            'quality': quality,
            'bpp': sum(values['bpp']) / len(values['bpp']),
            'ssimulacra2': sum(values['ssimulacra2']) / len(values['ssimulacra2']),
            'dssim': sum(values['dssim']) / len(values['dssim']),
        }
        if 'butteraugli' in values and values['butteraugli']:
            item['butteraugli'] = sum(values['butteraugli']) / len(values['butteraugli'])
        aggregated.append(item)

    return aggregated

def generate_svg(data, metric, title, ylabel, lower_is_better=False):
    """Generate SVG Pareto front plot with dark mode support."""
    rust_data = sorted([d for d in data if d['encoder'] == 'rust' and metric in d], key=lambda x: x['bpp'])
    c_data = sorted([d for d in data if d['encoder'] == 'c' and metric in d], key=lambda x: x['bpp'])

    if not rust_data or not c_data:
        return None

    # Determine plot bounds
    all_bpp = [d['bpp'] for d in rust_data + c_data]
    all_metric = [d[metric] for d in rust_data + c_data]

    min_bpp, max_bpp = min(all_bpp), max(all_bpp)
    min_metric, max_metric = min(all_metric), max(all_metric)

    # Add padding
    bpp_range = max_bpp - min_bpp
    metric_range = max_metric - min_metric
    min_bpp -= bpp_range * 0.05
    max_bpp += bpp_range * 0.05
    min_metric -= metric_range * 0.05
    max_metric += metric_range * 0.05

    # SVG dimensions
    width, height = 700, 450
    margin = {'top': 50, 'right': 140, 'bottom': 70, 'left': 90}
    plot_width = width - margin['left'] - margin['right']
    plot_height = height - margin['top'] - margin['bottom']

    def scale_x(v):
        return margin['left'] + (v - min_bpp) / (max_bpp - min_bpp) * plot_width

    def scale_y(v):
        if lower_is_better:
            return margin['top'] + (v - min_metric) / (max_metric - min_metric) * plot_height
        else:
            return margin['top'] + (1 - (v - min_metric) / (max_metric - min_metric)) * plot_height

    svg = []
    svg.append(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}">')

    # CSS with dark mode support
    svg.append('''<style>
  :root {
    --bg-color: #ffffff;
    --text-color: #1a1a1a;
    --grid-color: #e0e0e0;
    --axis-color: #333333;
    --legend-bg: #ffffff;
    --legend-border: #cccccc;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg-color: #1a1a1a;
      --text-color: #e0e0e0;
      --grid-color: #404040;
      --axis-color: #b0b0b0;
      --legend-bg: #2a2a2a;
      --legend-border: #505050;
    }
  }
  .background { fill: var(--bg-color); }
  .title { font: bold 18px system-ui, sans-serif; fill: var(--text-color); }
  .axis-label { font: 13px system-ui, sans-serif; fill: var(--text-color); }
  .tick-label { font: 11px system-ui, sans-serif; fill: var(--text-color); }
  .legend { font: 13px system-ui, sans-serif; fill: var(--text-color); }
  .grid { stroke: var(--grid-color); stroke-width: 1; }
  .axis { stroke: var(--axis-color); stroke-width: 1.5; }
  .rust-line { stroke: #e74c3c; stroke-width: 2.5; fill: none; }
  .c-line { stroke: #3498db; stroke-width: 2.5; fill: none; }
  .rust-point { fill: #e74c3c; }
  .c-point { fill: #3498db; }
  .legend-bg { fill: var(--legend-bg); stroke: var(--legend-border); }
</style>''')

    # Background
    svg.append(f'<rect class="background" width="{width}" height="{height}"/>')

    # Title
    svg.append(f'<text x="{width/2}" y="30" text-anchor="middle" class="title">{title}</text>')

    # Grid lines
    for i in range(6):
        bpp = min_bpp + i * (max_bpp - min_bpp) / 5
        x = scale_x(bpp)
        svg.append(f'<line x1="{x}" y1="{margin["top"]}" x2="{x}" y2="{height - margin["bottom"]}" class="grid"/>')

        val = min_metric + i * (max_metric - min_metric) / 5
        y = scale_y(val)
        svg.append(f'<line x1="{margin["left"]}" y1="{y}" x2="{width - margin["right"]}" y2="{y}" class="grid"/>')

    # Axes
    svg.append(f'<line x1="{margin["left"]}" y1="{height - margin["bottom"]}" '
               f'x2="{width - margin["right"]}" y2="{height - margin["bottom"]}" class="axis"/>')
    svg.append(f'<line x1="{margin["left"]}" y1="{margin["top"]}" '
               f'x2="{margin["left"]}" y2="{height - margin["bottom"]}" class="axis"/>')

    # Tick labels
    for i in range(6):
        bpp = min_bpp + i * (max_bpp - min_bpp) / 5
        x = scale_x(bpp)
        svg.append(f'<text x="{x}" y="{height - margin["bottom"] + 20}" text-anchor="middle" class="tick-label">{bpp:.2f}</text>')

        val = min_metric + i * (max_metric - min_metric) / 5
        y = scale_y(val)
        if metric == 'dssim':
            svg.append(f'<text x="{margin["left"] - 10}" y="{y + 4}" text-anchor="end" class="tick-label">{val:.5f}</text>')
        elif metric == 'butteraugli':
            svg.append(f'<text x="{margin["left"] - 10}" y="{y + 4}" text-anchor="end" class="tick-label">{val:.2f}</text>')
        else:
            svg.append(f'<text x="{margin["left"] - 10}" y="{y + 4}" text-anchor="end" class="tick-label">{val:.1f}</text>')

    # X axis label
    svg.append(f'<text x="{width/2}" y="{height - 20}" text-anchor="middle" class="axis-label">Bits per Pixel (BPP) →</text>')

    # Y axis label
    svg.append(f'<text x="25" y="{height/2}" text-anchor="middle" class="axis-label" '
               f'transform="rotate(-90 25 {height/2})">{ylabel}</text>')

    # Plot lines with smooth curves
    if rust_data:
        path = 'M ' + ' L '.join([f'{scale_x(d["bpp"]):.2f},{scale_y(d[metric]):.2f}' for d in rust_data])
        svg.append(f'<path d="{path}" class="rust-line"/>')
        for d in rust_data:
            svg.append(f'<circle cx="{scale_x(d["bpp"]):.2f}" cy="{scale_y(d[metric]):.2f}" r="5" class="rust-point"/>')

    if c_data:
        path = 'M ' + ' L '.join([f'{scale_x(d["bpp"]):.2f},{scale_y(d[metric]):.2f}' for d in c_data])
        svg.append(f'<path d="{path}" class="c-line"/>')
        for d in c_data:
            svg.append(f'<circle cx="{scale_x(d["bpp"]):.2f}" cy="{scale_y(d[metric]):.2f}" r="5" class="c-point"/>')

    # Legend
    legend_x = width - margin['right'] + 15
    legend_y = margin['top'] + 20
    svg.append(f'<rect x="{legend_x}" y="{legend_y - 15}" width="115" height="60" rx="4" class="legend-bg"/>')
    svg.append(f'<circle cx="{legend_x + 15}" cy="{legend_y + 5}" r="5" class="rust-point"/>')
    svg.append(f'<text x="{legend_x + 28}" y="{legend_y + 9}" class="legend">mozjpeg-oxide</text>')
    svg.append(f'<circle cx="{legend_x + 15}" cy="{legend_y + 30}" r="5" class="c-point"/>')
    svg.append(f'<text x="{legend_x + 28}" y="{legend_y + 34}" class="legend">C mozjpeg</text>')

    svg.append('</svg>')

    return '\n'.join(svg)

def main():
    if len(sys.argv) < 2:
        print("Usage: python plot_pareto.py benchmark_results.csv")
        sys.exit(1)

    csv_path = sys.argv[1]
    results = load_csv(csv_path)
    data = aggregate_by_quality(results)

    # Generate SSIMULACRA2 plot (higher is better)
    svg = generate_svg(data, 'ssimulacra2',
                       'mozjpeg-oxide vs C mozjpeg: Quality vs Size',
                       '← SSIMULACRA2 Score (higher is better)',
                       lower_is_better=False)
    if svg:
        with open('pareto_ssimulacra2.svg', 'w') as f:
            f.write(svg)
        print("Generated: pareto_ssimulacra2.svg")

    # Generate Butteraugli plot (lower is better)
    svg = generate_svg(data, 'butteraugli',
                       'mozjpeg-oxide vs C mozjpeg: Butteraugli',
                       'Butteraugli Score (lower is better) →',
                       lower_is_better=True)
    if svg:
        with open('pareto_butteraugli.svg', 'w') as f:
            f.write(svg)
        print("Generated: pareto_butteraugli.svg")

    # Generate DSSIM plot (lower is better)
    svg = generate_svg(data, 'dssim',
                       'mozjpeg-oxide vs C mozjpeg: DSSIM',
                       'DSSIM (lower is better) →',
                       lower_is_better=True)
    if svg:
        with open('pareto_dssim.svg', 'w') as f:
            f.write(svg)
        print("Generated: pareto_dssim.svg")

    # Print summary table
    print("\nSummary by Quality Level:")
    print("-" * 95)

    has_butteraugli = any('butteraugli' in d for d in data)
    if has_butteraugli:
        print(f"{'Q':>5} {'Rust BPP':>10} {'C BPP':>10} {'BPP Δ%':>10} {'Rust SSIM2':>12} {'C SSIM2':>12} {'Rust BA':>10} {'C BA':>10}")
    else:
        print(f"{'Q':>5} {'Rust BPP':>10} {'C BPP':>10} {'BPP Δ%':>10} {'Rust SSIM2':>12} {'C SSIM2':>12} {'SSIM2 Δ':>10}")
    print("-" * 95)

    qualities = sorted(set(d['quality'] for d in data))
    for q in qualities:
        rust = next((d for d in data if d['encoder'] == 'rust' and d['quality'] == q), None)
        c = next((d for d in data if d['encoder'] == 'c' and d['quality'] == q), None)
        if rust and c:
            bpp_diff = (rust['bpp'] - c['bpp']) / c['bpp'] * 100
            if has_butteraugli and 'butteraugli' in rust and 'butteraugli' in c:
                print(f"{q:>5} {rust['bpp']:>10.4f} {c['bpp']:>10.4f} {bpp_diff:>+9.2f}% "
                      f"{rust['ssimulacra2']:>12.2f} {c['ssimulacra2']:>12.2f} "
                      f"{rust['butteraugli']:>10.3f} {c['butteraugli']:>10.3f}")
            else:
                ssim_diff = rust['ssimulacra2'] - c['ssimulacra2']
                print(f"{q:>5} {rust['bpp']:>10.4f} {c['bpp']:>10.4f} {bpp_diff:>+9.2f}% "
                      f"{rust['ssimulacra2']:>12.2f} {c['ssimulacra2']:>12.2f} {ssim_diff:>+10.3f}")

if __name__ == '__main__':
    main()
