extends Container

@onready var buttonTemplate: Button = $ToggleChannel
@onready var playButton: Button = $PlayPause
var buttons: Array[Button] = []

func _ready():
	buttonTemplate.get_parent().remove_child(buttonTemplate)
	update(null)

func update(synth: Synthesizer):
	for button in buttons:
		button.queue_free()

	buttons = []

	if synth != null and synth.eval("typeof(synth) !== 'undefined' && synth.song != null"):
		playButton.synth = synth
		playButton.disabled = false
		var channels = synth.eval("synth.song.channels.length")
		for i in range(channels):
			var button: Button = buttonTemplate.duplicate()
			var instrumentName = synth.eval(
				"beepbox.Config.instrumentTypeNames[synth.song.channels[%d].instruments[0].type]" % i)
			button.active = synth.eval("synth.song.channels[%d].muted === false" % i)
			button.description = instrumentName
			button.index = i
			button.synth = synth
			button.update_text()
			buttons.append(button)
			add_child(button)
	else:
		playButton.disabled = true
