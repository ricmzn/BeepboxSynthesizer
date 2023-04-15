extends Button

@onready var synth: Synthesizer = $"/root/Main/Synthesizer"

var playing = false

func _on_Button_pressed():
	if playing:
		synth.pause()
		playing = false
		text = "Play"
	else:
		synth.resume()
		playing = true
		text = "Pause"
