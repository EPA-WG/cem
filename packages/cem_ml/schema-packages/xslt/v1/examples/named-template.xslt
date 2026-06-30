<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:template match="/">
    <section>default</section>
  </xsl:template>
  <xsl:template name="profile">
    <section class="profile">
      <p><xsl:value-of select="$label"/></p>
    </section>
  </xsl:template>
</xsl:stylesheet>
