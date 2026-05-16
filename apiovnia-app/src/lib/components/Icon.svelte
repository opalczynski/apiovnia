<!--
  Lucide-style icon primitive. We pass the inner SVG paths as a string (matching
  the design canvas IC dictionary) so the underlying icon set stays a single
  source of truth in $lib/components/icons.ts.
-->
<script lang="ts">
  type Props = {
    d: string;
    size?: number;
    strokeWidth?: number;
    class?: string;
  };

  const {
    d,
    size = 14,
    strokeWidth = 1.5,
    class: className = "",
  }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  class={className}
  style="display:block;flex-shrink:0"
>
  {@html d}
</svg>
