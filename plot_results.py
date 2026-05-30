import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
import numpy as np

# ── Data ──────────────────────────────────────────────────────────────────────
policies = ['FCFS', 'DES', 'DES\n+100ms', 'DES\n+500ms', 'DES\n+2000ms']
x = np.arange(len(policies))
width = 0.35

data = {
    'large_heavy': {
        'uncomp': [89.47, 96.58, 97.69, 98.26, 98.83],
        'comp':   [82.29, 88.85, 89.87, 90.39, 90.91],
        'blobs':  [190,   176,   174,   173,   172],
    },
    'small_heavy': {
        'uncomp': [77.49, 77.49, 77.49, 77.49, 77.49],
        'comp':   [50.00, 49.89, 49.89, 49.89, 49.88],
        'blobs':  [4, 4, 4, 4, 4],
    },
    'mixed': {
        'uncomp': [98.93, 98.93, 98.93, 98.93, 98.93],
        'comp':   [85.54, 85.52, 85.52, 85.52, 85.51],
        'blobs':  [16, 16, 16, 16, 16],
    },
}

colors_uncomp = ['#d62728', '#1f77b4', '#4a90d9', '#74b3e8', '#aed4f5']
colors_comp   = ['#e07b00', '#2ca02c', '#5bbf5b', '#88d488', '#b8e8b8']

dataset_labels = {
    'large_heavy': 'large_heavy.csv',
    'small_heavy': 'small_heavy.csv',
    'mixed':       'mixed.csv',
}

y_ranges = {
    'large_heavy': (78, 102),
    'small_heavy': (40, 90),
    'mixed':       (78, 104),
}

# ── Figure 1: Uncompressed Fill Rate ──────────────────────────────────────────
fig, axes = plt.subplots(1, 3, figsize=(16, 6))
fig.suptitle('Uncompressed Blob Fill Rate by Ordering Policy', fontsize=15, fontweight='bold', y=1.02)

for ax, (key, label) in zip(axes, dataset_labels.items()):
    d = data[key]
    bars = ax.bar(x, d['uncomp'], color=colors_uncomp, width=0.6, edgecolor='white', linewidth=0.8)

    # Blob count annotations above each bar
    for bar, blobs, val in zip(bars, d['blobs'], d['uncomp']):
        ax.text(bar.get_x() + bar.get_width() / 2, bar.get_height() + 0.3,
                f'{blobs}B\n{val:.1f}%', ha='center', va='bottom', fontsize=8, fontweight='bold')

    ax.set_title(label, fontsize=12, fontweight='bold')
    ax.set_xticks(x)
    ax.set_xticklabels(policies, fontsize=9)
    ax.set_ylabel('Uncompressed Fill Rate (%)', fontsize=10)
    ax.set_ylim(y_ranges[key])
    ax.yaxis.set_major_formatter(plt.FuncFormatter(lambda v, _: f'{v:.0f}%'))
    ax.grid(axis='y', linestyle='--', alpha=0.5)
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)

    # Highlight FCFS bar with a dashed border
    bars[0].set_edgecolor('#333333')
    bars[0].set_linewidth(2)
    bars[0].set_linestyle('--')

    # Improvement annotation arrow on large_heavy only
    if key == 'large_heavy':
        ax.annotate('', xy=(4, d['uncomp'][4]), xytext=(0, d['uncomp'][0]),
                    arrowprops=dict(arrowstyle='<->', color='black', lw=1.5,
                                   connectionstyle='arc3,rad=0.0'))
        mid_x = 2.0
        mid_y = (d['uncomp'][0] + d['uncomp'][4]) / 2 + 1.5
        ax.text(mid_x, mid_y, f'+{d["uncomp"][4] - d["uncomp"][0]:.2f}pp',
                ha='center', fontsize=10, fontweight='bold', color='black')

plt.tight_layout()
plt.savefig('fill_rate_uncompressed.png', dpi=150, bbox_inches='tight')
print("Saved: fill_rate_uncompressed.png")

# ── Figure 2: Compressed Fill Rate ────────────────────────────────────────────
y_ranges_comp = {
    'large_heavy': (75, 96),
    'small_heavy': (35, 65),
    'mixed':       (78, 92),
}

fig2, axes2 = plt.subplots(1, 3, figsize=(16, 6))
fig2.suptitle('Compressed Blob Fill Rate by Ordering Policy', fontsize=15, fontweight='bold', y=1.02)

for ax, (key, label) in zip(axes2, dataset_labels.items()):
    d = data[key]
    bars = ax.bar(x, d['comp'], color=colors_comp, width=0.6, edgecolor='white', linewidth=0.8)

    for bar, blobs, val in zip(bars, d['blobs'], d['comp']):
        ax.text(bar.get_x() + bar.get_width() / 2, bar.get_height() + 0.3,
                f'{blobs}B\n{val:.1f}%', ha='center', va='bottom', fontsize=8, fontweight='bold')

    ax.set_title(label, fontsize=12, fontweight='bold')
    ax.set_xticks(x)
    ax.set_xticklabels(policies, fontsize=9)
    ax.set_ylabel('Compressed Fill Rate (%)', fontsize=10)
    ax.set_ylim(y_ranges_comp[key])
    ax.yaxis.set_major_formatter(plt.FuncFormatter(lambda v, _: f'{v:.0f}%'))
    ax.grid(axis='y', linestyle='--', alpha=0.5)
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)

    bars[0].set_edgecolor('#333333')
    bars[0].set_linewidth(2)
    bars[0].set_linestyle('--')

    if key == 'large_heavy':
        ax.annotate('', xy=(4, d['comp'][4]), xytext=(0, d['comp'][0]),
                    arrowprops=dict(arrowstyle='<->', color='black', lw=1.5))
        mid_y = (d['comp'][0] + d['comp'][4]) / 2 + 1.5
        ax.text(2.0, mid_y, f'+{d["comp"][4] - d["comp"][0]:.2f}pp',
                ha='center', fontsize=10, fontweight='bold', color='black')

plt.tight_layout()
plt.savefig('fill_rate_compressed.png', dpi=150, bbox_inches='tight')
print("Saved: fill_rate_compressed.png")

# ── Figure 3: large_heavy zoomed — most important graph ───────────────────────
fig3, ax3 = plt.subplots(figsize=(10, 6))
ax3.set_title('Blob Fill Rate — large_heavy.csv (Zoomed)', fontsize=14, fontweight='bold')

d = data['large_heavy']
bars_u = ax3.bar(x - width/2, d['uncomp'], width, label='Uncompressed', color=colors_uncomp, edgecolor='white')
bars_c = ax3.bar(x + width/2, d['comp'],   width, label='Compressed',   color=colors_comp,   edgecolor='white')

for bar, val in zip(bars_u, d['uncomp']):
    ax3.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.2,
             f'{val:.1f}%', ha='center', va='bottom', fontsize=9, fontweight='bold')

for bar, val in zip(bars_c, d['comp']):
    ax3.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.2,
             f'{val:.1f}%', ha='center', va='bottom', fontsize=9, fontweight='bold')

ax3.set_xticks(x)
ax3.set_xticklabels(policies, fontsize=10)
ax3.set_ylabel('Fill Rate (%)', fontsize=11)
ax3.set_ylim(78, 103)
ax3.yaxis.set_major_formatter(plt.FuncFormatter(lambda v, _: f'{v:.0f}%'))
ax3.grid(axis='y', linestyle='--', alpha=0.5)
ax3.spines['top'].set_visible(False)
ax3.spines['right'].set_visible(False)
ax3.legend(fontsize=10)

# Highlight FCFS
bars_u[0].set_edgecolor('#333333')
bars_u[0].set_linewidth(2)
bars_c[0].set_edgecolor('#333333')
bars_c[0].set_linewidth(2)

# Blob count below x-axis
for i, blobs in enumerate(d['blobs']):
    ax3.text(i, 78.5, f'{blobs} blobs', ha='center', fontsize=8, color='#444444')

plt.tight_layout()
plt.savefig('fill_rate_large_heavy_zoomed.png', dpi=150, bbox_inches='tight')
print("Saved: fill_rate_large_heavy_zoomed.png")

plt.show()

# ── Figure 4: Wasted Space Rate — FCFS vs DES (Optimized Weights) ─────────────
# Inverted metric: 100% - fill rate = wasted blob space
# FCFS is the tall "bad" bar; optimized DES is the shortest bar.
policies_opt = ['FCFS', 'DES', 'DES\n+100ms', 'DES\n+500ms', 'DES\n+2000ms', 'DES+2000ms\n(α=0.1,γ=0.8)']
x_opt = np.arange(len(policies_opt))

uncomp_fill = [89.47, 96.58, 97.69, 98.26, 98.83, 99.41]
comp_fill   = [82.29, 88.85, 89.87, 90.39, 90.91, 91.45]
blobs_opt   = [190,   176,   174,   173,   172,   171]

uncomp_waste = [100 - v for v in uncomp_fill]
comp_waste   = [100 - v for v in comp_fill]

colors_uncomp_waste = ['#d62728', '#1f77b4', '#4a90d9', '#74b3e8', '#aed4f5', '#2c7a2c']
colors_comp_waste   = ['#e07b00', '#2ca02c', '#5bbf5b', '#88d488', '#b8e8b8', '#1a5c1a']

fig4, ax4 = plt.subplots(figsize=(13, 6))
ax4.set_title('Wasted Blob Space — large_heavy.csv  |  FCFS vs DES (Optimized Weights)', fontsize=13, fontweight='bold')

bars_u4 = ax4.bar(x_opt - width/2, uncomp_waste, width, label='Uncompressed waste', color=colors_uncomp_waste, edgecolor='white')
bars_c4 = ax4.bar(x_opt + width/2, comp_waste,   width, label='Compressed waste',   color=colors_comp_waste,   edgecolor='white')

for bar, val in zip(bars_u4, uncomp_waste):
    ax4.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.15,
             f'{val:.2f}%', ha='center', va='bottom', fontsize=8.5, fontweight='bold')

for bar, val in zip(bars_c4, comp_waste):
    ax4.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.15,
             f'{val:.2f}%', ha='center', va='bottom', fontsize=8.5, fontweight='bold')

ax4.set_xticks(x_opt)
ax4.set_xticklabels(policies_opt, fontsize=9.5)
ax4.set_ylabel('Wasted Space (%)', fontsize=11)
ax4.set_ylim(0, 22)
ax4.yaxis.set_major_formatter(plt.FuncFormatter(lambda v, _: f'{v:.0f}%'))
ax4.grid(axis='y', linestyle='--', alpha=0.5)
ax4.spines['top'].set_visible(False)
ax4.spines['right'].set_visible(False)
ax4.legend(fontsize=10)

# Highlight FCFS with dashed border (the "bad" baseline)
for bar in [bars_u4[0], bars_c4[0]]:
    bar.set_edgecolor('#333333')
    bar.set_linewidth(2)
    bar.set_linestyle('--')

# Highlight optimized bar with gold border (the best result)
for bar in [bars_u4[5], bars_c4[5]]:
    bar.set_edgecolor('#b8860b')
    bar.set_linewidth(2.5)

# Blob count above each group
for i, blobs in enumerate(blobs_opt):
    ax4.text(i, max(uncomp_waste[i], comp_waste[i]) + 1.1,
             f'{blobs}B', ha='center', fontsize=8, color='#444444', fontweight='bold')

# Reduction arrows: FCFS → optimized, pointing downward
ax4.annotate('', xy=(5, uncomp_waste[5]), xytext=(0, uncomp_waste[0]),
             arrowprops=dict(arrowstyle='<->', color='#b8860b', lw=2.0,
                             connectionstyle='arc3,rad=0.0'))
mid_y_u = (uncomp_waste[0] + uncomp_waste[5]) / 2 + 0.4
ax4.text(2.5, mid_y_u, f'−{uncomp_waste[0] - uncomp_waste[5]:.2f}pp uncomp waste',
         ha='center', fontsize=9.5, fontweight='bold', color='#b8860b')

ax4.annotate('', xy=(5, comp_waste[5]), xytext=(0, comp_waste[0]),
             arrowprops=dict(arrowstyle='<->', color='#555555', lw=1.5,
                             connectionstyle='arc3,rad=0.0'))
mid_y_c = (comp_waste[0] + comp_waste[5]) / 2 - 0.8
ax4.text(2.5, mid_y_c, f'−{comp_waste[0] - comp_waste[5]:.2f}pp comp waste',
         ha='center', fontsize=9.5, fontweight='bold', color='#555555')

plt.tight_layout()
plt.savefig('fill_rate_optimized.png', dpi=150, bbox_inches='tight')
print("Saved: fill_rate_optimized.png")
