const name = 'i3stat';
const debug = './target/debug';
export enum Bin {
	i3Stat = `${debug}/${name}`,
	i3StatIpc = `${debug}/${name}-ipc`,
	i3StatSignals = `${debug}/${name}-signals`,
	i3StatSensors = `${debug}/${name}-sensors`,
}

export const superscript = {
	'0': '⁰',
	'1': '¹',
	'2': '²',
	'3': '³',
	'4': '⁴',
	'5': '⁵',
	'6': '⁶',
	'7': '⁷',
	'8': '⁸',
	'9': '⁹',
	'+': '⁺',
	'-': '⁻',
	'=': '⁼',
	'(': '⁽',
	')': '⁾',
	'a': 'ᵃ',
	'b': 'ᵇ',
	'c': 'ᶜ',
	'd': 'ᵈ',
	'e': 'ᵉ',
	'f': 'ᶠ',
	'g': 'ᵍ',
	'h': 'ʰ',
	'i': 'ⁱ',
	'j': 'ʲ',
	'k': 'ᵏ',
	'l': 'ˡ',
	'm': 'ᵐ',
	'n': 'ⁿ',
	'o': 'ᵒ',
	'p': 'ᵖ',
	'r': 'ʳ',
	's': 'ˢ',
	't': 'ᵗ',
	'u': 'ᵘ',
	'v': 'ᵛ',
	'w': 'ʷ',
	'x': 'ˣ',
	'y': 'ʸ',
	'z': 'ᶻ',
	' ': ' '
};

export const subscript = {
	'0': '₀',
	'1': '₁',
	'2': '₂',
	'3': '₃',
	'4': '₄',
	'5': '₅',
	'6': '₆',
	'7': '₇',
	'8': '₈',
	'9': '₉',
	'+': '₊',
	'-': '₋',
	'=': '₌',
	'(': '₍',
	')': '₎',
	'a': 'ₐ',
	'e': 'ₑ',
	'h': 'ₕ',
	'i': 'ᵢ',
	'j': 'ⱼ',
	'k': 'ₖ',
	'l': 'ₗ',
	'm': 'ₘ',
	'n': 'ₙ',
	'o': 'ₒ',
	'p': 'ₚ',
	'r': 'ᵣ',
	's': 'ₛ',
	't': 'ₜ',
	'u': 'ᵤ',
	'v': 'ᵥ',
	'x': 'ₓ',
	' ': ' '
};