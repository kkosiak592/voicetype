Add-Type -AssemblyName System.Speech

$OutputDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $OutputDir) { $OutputDir = "." }

# 16kHz, 16-bit, mono PCM — the format whisper-rs and parakeet-rs expect
# SpeechAudioFormatInfo(samplesPerSecond, bitsPerSample, channel)
$format = [System.Speech.AudioFormat.SpeechAudioFormatInfo]::new(
    16000,  # samples per second
    [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen,
    [System.Speech.AudioFormat.AudioChannel]::Mono
)

$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer

# -----------------------------------------------------------------------
# 5-second clip — short phrase for quick-latency testing
# -----------------------------------------------------------------------
$file5s = Join-Path $OutputDir "benchmark-5s.wav"
$synth.SetOutputToWaveFile($file5s, $format)
$synth.Speak("The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs.")
$synth.SetOutputToNull()

$size5s = (Get-Item $file5s).Length
Write-Host ("benchmark-5s.wav  -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size5s, ($size5s / (16000 * 2)))

# -----------------------------------------------------------------------
# 30-second clip — medium passage for mid-length testing
# -----------------------------------------------------------------------
$passage30s = @"
Speech recognition technology has advanced significantly in recent years.
Modern deep learning models can transcribe audio with remarkable accuracy.
The key factors that affect performance include microphone quality and background noise.
Models trained on large datasets tend to generalize better across different speakers.
Quantized models offer a good balance between speed and accuracy for real time use.
This thirty second clip tests how models handle medium length audio segments.
"@

$file30s = Join-Path $OutputDir "benchmark-30s.wav"
$synth.SetOutputToWaveFile($file30s, $format)
$synth.Speak($passage30s)
$synth.SetOutputToNull()

$size30s = (Get-Item $file30s).Length
Write-Host ("benchmark-30s.wav -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size30s, ($size30s / (16000 * 2)))

# -----------------------------------------------------------------------
# 60-second clip — longer passage for sustained-load testing
# -----------------------------------------------------------------------
$passage = @"
Voice dictation software converts spoken words into written text in real time.
Modern systems use deep learning models trained on thousands of hours of audio data.
Accuracy depends on microphone quality, background noise, and speaking pace.
The whisper model was released by OpenAI and is widely used for offline transcription.
Parakeet is an NVIDIA model optimised for real-time inference on CUDA hardware.
To benchmark these models fairly, we measure wall-clock latency across multiple runs.
We test both a short five-second clip and a longer sixty-second passage.
Results include the average, minimum, and maximum inference time in milliseconds.
Lower latency means faster transcription and a better user experience.
Sub five hundred millisecond latency is generally imperceptible to the user.
English language models tend to be smaller and faster than multilingual alternatives.
Quantised models use reduced precision weights to run faster with minimal accuracy loss.
The Q5 underscore 1 format stores each weight in approximately five bits.
GPU acceleration can reduce inference time by ten times compared to CPU-only execution.
This benchmark helps select the best model for a given hardware configuration.
"@

$file60s = Join-Path $OutputDir "benchmark-60s.wav"
$synth.SetOutputToWaveFile($file60s, $format)
$synth.Speak($passage)
$synth.SetOutputToNull()

$size60s = (Get-Item $file60s).Length
Write-Host ("benchmark-60s.wav -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size60s, ($size60s / (16000 * 2)))

# -----------------------------------------------------------------------
# 5-second clip variant B — copper wire / circuit board
# -----------------------------------------------------------------------
$file5sb = Join-Path $OutputDir "benchmark-5s-b.wav"
$synth.SetOutputToWaveFile($file5sb, $format)
$synth.Speak("A copper wire carries electrical current through the circuit board with minimal resistance.")
$synth.SetOutputToNull()

$size5sb = (Get-Item $file5sb).Length
Write-Host ("benchmark-5s-b.wav  -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size5sb, ($size5sb / (16000 * 2)))

# -----------------------------------------------------------------------
# 5-second clip variant C — satellite orbit
# -----------------------------------------------------------------------
$file5sc = Join-Path $OutputDir "benchmark-5s-c.wav"
$synth.SetOutputToWaveFile($file5sc, $format)
$synth.Speak("The satellite orbits Earth every ninety minutes, capturing high resolution photographs.")
$synth.SetOutputToNull()

$size5sc = (Get-Item $file5sc).Length
Write-Host ("benchmark-5s-c.wav  -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size5sc, ($size5sc / (16000 * 2)))

# -----------------------------------------------------------------------
# 30-second clip variant B — steel manufacturing
# -----------------------------------------------------------------------
$passage30sb = @"
The process of steel manufacturing begins with iron ore extraction from open pit mines.
Workers transport the raw material to blast furnaces where temperatures exceed fifteen hundred degrees.
Carbon is introduced to create an alloy stronger than pure iron alone.
Rolling mills then shape the molten steel into beams, sheets, and coiled wire.
Quality control inspectors test samples for tensile strength and corrosion resistance.
Modern foundries produce over two billion tonnes of steel worldwide each year.
"@

$file30sb = Join-Path $OutputDir "benchmark-30s-b.wav"
$synth.SetOutputToWaveFile($file30sb, $format)
$synth.Speak($passage30sb)
$synth.SetOutputToNull()

$size30sb = (Get-Item $file30sb).Length
Write-Host ("benchmark-30s-b.wav -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size30sb, ($size30sb / (16000 * 2)))

# -----------------------------------------------------------------------
# 30-second clip variant C — Mediterranean cooking
# -----------------------------------------------------------------------
$passage30sc = @"
Mediterranean cooking relies heavily on olive oil, fresh herbs, and seasonal vegetables.
Tomatoes were introduced to European cuisine after Spanish explorers returned from the Americas.
A traditional risotto requires constant stirring to release starch from Arborio rice grains.
Fermentation transforms grape juice into wine through the action of natural yeasts on sugar.
Sourdough bread uses a live culture of bacteria and wild yeast instead of commercial packets.
The Maillard reaction between amino acids and sugars creates the brown crust on grilled meat.
"@

$file30sc = Join-Path $OutputDir "benchmark-30s-c.wav"
$synth.SetOutputToWaveFile($file30sc, $format)
$synth.Speak($passage30sc)
$synth.SetOutputToNull()

$size30sc = (Get-Item $file30sc).Length
Write-Host ("benchmark-30s-c.wav -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size30sc, ($size30sc / (16000 * 2)))

# -----------------------------------------------------------------------
# 60-second clip variant B — Panama Canal
# -----------------------------------------------------------------------
$passage60sb = @"
The Panama Canal connects the Atlantic and Pacific oceans through a series of concrete locks.
Construction began in nineteen oh four and took ten years to complete at enormous human cost.
Ships entering from the Atlantic side are raised twenty six metres above sea level by three lock chambers.
Gatun Lake was created by damming the Chagres River and flooding an entire valley.
Each lock chamber uses gravity fed water from the lake rather than mechanical pumps.
A single transit moves approximately two hundred million litres of fresh water into the ocean.
The canal was expanded in twenty sixteen with larger locks to accommodate modern container ships.
These new Neopanamax locks use water saving basins that recycle sixty percent of each fill.
Over fourteen thousand vessels pass through the canal annually carrying five percent of world trade.
Drought conditions in recent years have forced authorities to limit daily transits and vessel draft.
Tolls range from a few hundred dollars for small sailboats to nearly a million for the largest tankers.
The canal remains one of the most significant engineering achievements of the twentieth century.
Ongoing maintenance requires continuous dredging of the navigational channel to prevent silting.
Tropical rainfall patterns directly influence water levels in Gatun and Alajuela lakes.
The Panama Canal Authority employs over nine thousand workers to operate and maintain the waterway.
"@

$file60sb = Join-Path $OutputDir "benchmark-60s-b.wav"
$synth.SetOutputToWaveFile($file60sb, $format)
$synth.Speak($passage60sb)
$synth.SetOutputToNull()

$size60sb = (Get-Item $file60sb).Length
Write-Host ("benchmark-60s-b.wav -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size60sb, ($size60sb / (16000 * 2)))

# -----------------------------------------------------------------------
# 60-second clip variant C — human immune system
# -----------------------------------------------------------------------
$passage60sc = @"
The human immune system consists of two complementary defence mechanisms working in coordination.
Innate immunity provides immediate but non specific protection through physical barriers and white blood cells.
Neutrophils are the first responders arriving at infection sites within minutes of tissue damage.
The adaptive immune system develops targeted responses through B cells and T cells over several days.
B cells produce antibodies that bind to specific molecular patterns on the surface of pathogens.
Helper T cells coordinate the overall immune response by releasing chemical signalling molecules called cytokines.
Memory cells persist for decades allowing the body to mount rapid responses to previously encountered threats.
Vaccination works by introducing harmless fragments of a pathogen to train the adaptive immune system.
Autoimmune disorders occur when the immune system mistakenly attacks the body's own healthy tissue.
Allergic reactions represent an exaggerated immune response to normally harmless environmental substances.
Immunosuppressive drugs are prescribed after organ transplants to prevent rejection of donor tissue.
The thymus gland plays a critical role in T cell maturation during childhood and adolescence.
Researchers continue developing immunotherapy treatments that harness the immune system to fight cancer cells.
The gut microbiome influences immune function through constant interaction with intestinal immune tissue.
Regular moderate exercise has been shown to enhance immune surveillance and reduce inflammation markers.
"@

$file60sc = Join-Path $OutputDir "benchmark-60s-c.wav"
$synth.SetOutputToWaveFile($file60sc, $format)
$synth.Speak($passage60sc)
$synth.SetOutputToNull()

$size60sc = (Get-Item $file60sc).Length
Write-Host ("benchmark-60s-c.wav -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size60sc, ($size60sc / (16000 * 2)))

# -----------------------------------------------------------------------
# 90-second clip — deep-sea oceanography
# -----------------------------------------------------------------------
$passage90s = @"
The deep ocean covers more than half of Earth's surface yet remains one of the least explored environments on the planet.
Hydrothermal vents were first discovered in nineteen seventy eight along the Galapagos Rift near the equatorial Pacific.
These vents release superheated water rich in minerals dissolved from the underlying oceanic crust.
Temperatures near black smoker chimneys can exceed three hundred and fifty degrees Celsius.
Despite the extreme heat, pressure, and complete absence of sunlight, dense ecosystems thrive around the vents.
Chemosynthetic bacteria form the base of these food webs by oxidising hydrogen sulphide instead of performing photosynthesis.
Giant tube worms can grow over two metres long and harbour symbiotic bacteria in a specialised organ called the trophosome.
Deep sea fish have evolved low density tissues and flexible bodies to withstand these crushing conditions.
The anglerfish uses a bioluminescent lure dangling from its head to attract prey in total darkness.
The Mariana Trench in the western Pacific reaches a depth of nearly eleven kilometres at Challenger Deep.
Researchers use remotely operated vehicles equipped with cameras and robotic arms to collect biological and mineral samples.
Sound travels faster and farther in cold deep water, enabling blue whales to communicate across entire ocean basins.
"@

$file90s = Join-Path $OutputDir "benchmark-90s.wav"
$synth.SetOutputToWaveFile($file90s, $format)
$synth.Speak($passage90s)
$synth.SetOutputToNull()

$size90s = (Get-Item $file90s).Length
Write-Host ("benchmark-90s.wav -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size90s, ($size90s / (16000 * 2)))

# -----------------------------------------------------------------------
# 90-second clip variant B — history of aviation
# -----------------------------------------------------------------------
$passage90sb = @"
The history of powered flight began on a cold December morning in nineteen oh three at Kitty Hawk, North Carolina.
Orville and Wilbur Wright achieved the first sustained controlled flight lasting twelve seconds and covering thirty seven metres.
Their success depended on three years of systematic experiments with kites, gliders, and wind tunnel models.
Early aircraft were constructed from spruce wood, cotton fabric, and bicycle chain drives connected to pusher propellers.
World War One accelerated aviation technology as military commanders recognised the strategic value of aerial reconnaissance.
Charles Lindbergh crossed the Atlantic solo in nineteen twenty seven, completing the thirty three hour journey in the Spirit of St. Louis.
Frank Whittle in Britain and Hans von Ohain in Germany independently developed the jet engine in the late nineteen thirties.
Breaking the sound barrier in October nineteen forty seven, Chuck Yeager flew the Bell X-one to Mach one point oh six.
The Boeing seven oh seven entered commercial service in nineteen fifty eight, making transatlantic travel accessible to millions.
Concorde, the supersonic passenger jet developed jointly by Britain and France, cruised at twice the speed of sound.
Composite materials including carbon fibre reinforced polymers now make up more than half of the structural weight of new airliners.
More than four billion passengers board commercial flights each year, making aviation one of the most transformative technologies in human history.
"@

$file90sb = Join-Path $OutputDir "benchmark-90s-b.wav"
$synth.SetOutputToWaveFile($file90sb, $format)
$synth.Speak($passage90sb)
$synth.SetOutputToNull()

$size90sb = (Get-Item $file90sb).Length
Write-Host ("benchmark-90s-b.wav -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size90sb, ($size90sb / (16000 * 2)))

# -----------------------------------------------------------------------
# 90-second clip variant C — renewable energy systems
# -----------------------------------------------------------------------
$passage90sc = @"
Renewable energy systems harness naturally replenishing resources to generate electricity without depleting finite fossil fuel reserves.
Solar photovoltaic cells convert sunlight directly into electrical current through the photoelectric effect discovered by Albert Einstein.
Silicon wafers doped with phosphorus and boron create a semiconductor junction that releases electrons when struck by photons.
The cost of solar panels has fallen by more than ninety percent over the past fifteen years due to manufacturing scale and efficiency gains.
Wind turbines extract kinetic energy from moving air masses through rotating blades connected to an electrical generator.
Offshore wind installations benefit from stronger and more consistent wind speeds than land based sites, increasing capacity factors.
Modern turbines stand over two hundred metres tall with blades spanning more than one hundred metres tip to tip.
Grid scale battery storage using lithium ion or iron phosphate chemistry allows excess renewable generation to be shifted to peak demand periods.
Pumped hydro storage remains the largest form of grid storage globally, using surplus electricity to pump water uphill into reservoirs.
Geothermal energy taps heat stored in rock and water beneath the Earth's surface to generate baseload electricity.
The levelised cost of energy from onshore wind and utility scale solar now undercuts new coal and gas plants in most markets.
International energy agencies project that renewables could supply more than eighty percent of global electricity by twenty fifty.
"@

$file90sc = Join-Path $OutputDir "benchmark-90s-c.wav"
$synth.SetOutputToWaveFile($file90sc, $format)
$synth.Speak($passage90sc)
$synth.SetOutputToNull()

$size90sc = (Get-Item $file90sc).Length
Write-Host ("benchmark-90s-c.wav -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size90sc, ($size90sc / (16000 * 2)))

$synth.Dispose()
Write-Host "Done. 12 WAV files written to: $OutputDir"
