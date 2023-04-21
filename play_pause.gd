extends Button

@onready var synth: Synthesizer = %Synthesizer

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
