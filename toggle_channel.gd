extends Button

@onready var initial_text = text

var player: AudioStreamPlayer
var description = ""
var active = true
var index = 0

func update_text():
	text = "Toggle Channel %d" % index
	tooltip_text = description
	if not active:
		text = "%s [M]" % text

func _ready():
	update_text()

func _on_Button_pressed():
	if active:
		player.get_stream_playback().eval("synth.song.channels[%d].muted = true" % index)
		active = false
		update_text()
	else:
		player.get_stream_playback().eval("synth.song.channels[%d].muted = false" % index)
		active = true
		update_text()
